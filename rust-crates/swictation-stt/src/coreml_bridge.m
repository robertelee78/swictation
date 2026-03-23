#import <CoreML/CoreML.h>
#import <Foundation/Foundation.h>
#include <string.h>
#include "coreml_bridge.h"

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

static char* make_error(NSError* error) {
    if (!error) return NULL;
    const char* desc = [[error localizedDescription] UTF8String];
    if (!desc) return NULL;
    return strdup(desc);
}

static char* make_error_str(const char* msg) {
    if (!msg) return NULL;
    return strdup(msg);
}

static void set_error(char** error_out, char* msg) {
    if (error_out) {
        *error_out = msg;
    } else {
        free(msg);
    }
}

/// Compute strides for a contiguous row-major array given its shape.
static NSArray<NSNumber*>* compute_strides(const int64_t* shape, int ndims) {
    NSMutableArray<NSNumber*>* strides = [NSMutableArray arrayWithCapacity:ndims];
    int64_t stride = 1;
    for (int i = ndims - 1; i >= 0; i--) {
        [strides insertObject:@(stride) atIndex:0];
        stride *= shape[i];
    }
    return strides;
}

/// Create an MLMultiArray wrapping an external float buffer (zero-copy).
/// The caller retains ownership of data_ptr; deallocator is nil.
static MLMultiArray* make_multiarray(const float* data_ptr,
                                     const int64_t* shape,
                                     int ndims,
                                     NSError** ns_error) {
    NSMutableArray<NSNumber*>* shapeArr = [NSMutableArray arrayWithCapacity:ndims];
    for (int i = 0; i < ndims; i++) {
        [shapeArr addObject:@(shape[i])];
    }

    NSArray<NSNumber*>* strides = compute_strides(shape, ndims);

    MLMultiArray* array = [[MLMultiArray alloc]
        initWithDataPointer:(void*)data_ptr
                      shape:shapeArr
                   dataType:MLMultiArrayDataTypeFloat32
                    strides:strides
                deallocator:nil
                      error:ns_error];
    return array;
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

coreml_model_t coreml_load_model(const char* mlmodelc_path,
                                  int compute_units,
                                  char** error_out) {
    @autoreleasepool {
        if (!mlmodelc_path) {
            set_error(error_out, make_error_str("mlmodelc_path is NULL"));
            return NULL;
        }

        NSString* pathStr = [NSString stringWithUTF8String:mlmodelc_path];
        NSURL* modelURL = [NSURL fileURLWithPath:pathStr];

        MLModelConfiguration* config = [[MLModelConfiguration alloc] init];
        switch (compute_units) {
            case 0:
                config.computeUnits = MLComputeUnitsCPUOnly;
                break;
            case 1:
                config.computeUnits = MLComputeUnitsCPUAndGPU;
                break;
            case 2:
                config.computeUnits = MLComputeUnitsAll;
                break;
            case 3:
                config.computeUnits = MLComputeUnitsCPUAndNeuralEngine;
                break;
            default:
                config.computeUnits = MLComputeUnitsAll;
                break;
        }

        NSError* ns_error = nil;
        MLModel* model = [MLModel modelWithContentsOfURL:modelURL
                                           configuration:config
                                                   error:&ns_error];
        if (!model) {
            set_error(error_out, make_error(ns_error));
            return NULL;
        }

        // Prevent ARC from releasing the model once we hand it out as void*.
        return (__bridge_retained void*)model;
    }
}

int coreml_predict(coreml_model_t handle,
                   const char* input_name,
                   const float* input_data,
                   const int64_t* input_shape,
                   int input_ndims,
                   const char* output_name,
                   float* output_data,
                   int64_t output_size,
                   char** error_out) {
    // Delegate to the multi-input version with a single input.
    const char* names[] = { input_name };
    const float* datas[] = { input_data };
    const int64_t* shapes[] = { input_shape };
    int ndims_arr[] = { input_ndims };

    return coreml_predict_multi(handle,
                                names, datas, shapes, ndims_arr, 1,
                                output_name, output_data, output_size,
                                error_out);
}

int coreml_predict_multi(coreml_model_t handle,
                         const char** input_names,
                         const float** input_datas,
                         const int64_t** input_shapes,
                         const int* input_ndims_arr,
                         int num_inputs,
                         const char* output_name,
                         float* output_data,
                         int64_t output_size,
                         char** error_out) {
    @autoreleasepool {
        if (!handle) {
            set_error(error_out, make_error_str("model handle is NULL"));
            return -1;
        }
        if (!output_data || output_size <= 0) {
            set_error(error_out, make_error_str("output_data is NULL or output_size <= 0"));
            return -1;
        }

        MLModel* model = (__bridge MLModel*)handle;
        NSError* ns_error = nil;

        // Build input feature dictionary.
        NSMutableDictionary* featureDict = [NSMutableDictionary dictionaryWithCapacity:num_inputs];
        for (int i = 0; i < num_inputs; i++) {
            if (!input_names[i] || !input_datas[i] || !input_shapes[i]) {
                set_error(error_out, make_error_str("NULL input parameter"));
                return -1;
            }

            MLMultiArray* array = make_multiarray(input_datas[i],
                                                  input_shapes[i],
                                                  input_ndims_arr[i],
                                                  &ns_error);
            if (!array) {
                set_error(error_out, make_error(ns_error));
                return -1;
            }

            NSString* name = [NSString stringWithUTF8String:input_names[i]];
            featureDict[name] = [MLFeatureValue featureValueWithMultiArray:array];
        }

        MLDictionaryFeatureProvider* provider =
            [[MLDictionaryFeatureProvider alloc] initWithDictionary:featureDict
                                                             error:&ns_error];
        if (!provider) {
            set_error(error_out, make_error(ns_error));
            return -1;
        }

        // Run prediction.
        id<MLFeatureProvider> result = [model predictionFromFeatures:provider
                                                              error:&ns_error];
        if (!result) {
            set_error(error_out, make_error(ns_error));
            return -1;
        }

        // Extract output.
        NSString* outName = [NSString stringWithUTF8String:output_name];
        MLFeatureValue* outFeature = [result featureValueForName:outName];
        if (!outFeature || !outFeature.multiArrayValue) {
            set_error(error_out, make_error_str("output feature not found or not a multi-array"));
            return -1;
        }

        MLMultiArray* outArray = outFeature.multiArrayValue;
        int64_t totalCount = outArray.count;
        int64_t copyCount = (totalCount < output_size) ? totalCount : output_size;

        // Copy output data. Access the underlying buffer directly.
        const float* src = (const float*)outArray.dataPointer;
        memcpy(output_data, src, (size_t)copyCount * sizeof(float));

        return 0;
    }
}

void coreml_free_model(coreml_model_t handle) {
    if (!handle) return;
    @autoreleasepool {
        // Transfer ownership back to ARC, which will release the model.
        MLModel* model __attribute__((unused)) = (__bridge_transfer MLModel*)handle;
    }
}

void coreml_free_string(char* str) {
    free(str);
}
