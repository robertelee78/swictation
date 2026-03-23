#ifndef COREML_BRIDGE_H
#define COREML_BRIDGE_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Opaque handle to a loaded CoreML model */
typedef void* coreml_model_t;

/**
 * Load a compiled CoreML model (.mlmodelc directory).
 *
 * @param mlmodelc_path  Filesystem path to the .mlmodelc directory.
 * @param compute_units  0 = CPU only, 1 = CPU+GPU, 2 = CPU+GPU+ANE (all), 3 = CPU+ANE.
 * @param error_out      If non-NULL and an error occurs, receives a strdup'd error string.
 *                       Caller must free with coreml_free_string().
 * @return Opaque model handle, or NULL on failure.
 */
coreml_model_t coreml_load_model(const char* mlmodelc_path,
                                  int compute_units,
                                  char** error_out);

/**
 * Run single-input prediction on a CoreML model.
 *
 * @param model        Handle from coreml_load_model().
 * @param input_name   The model's input feature name (e.g. "audio_signal").
 * @param input_data   Pointer to float32 array (owned by caller).
 * @param input_shape  Array of dimension sizes (e.g. {1, 128, 500}).
 * @param input_ndims  Number of dimensions in input_shape.
 * @param output_name  The model's output feature name.
 * @param output_data  Pre-allocated float32 output buffer (owned by caller).
 * @param output_size  Size of output buffer in number of floats.
 * @param error_out    If non-NULL, receives error string on failure.
 * @return 0 on success, -1 on failure.
 */
int coreml_predict(coreml_model_t model,
                   const char* input_name,
                   const float* input_data,
                   const int64_t* input_shape,
                   int input_ndims,
                   const char* output_name,
                   float* output_data,
                   int64_t output_size,
                   char** error_out);

/**
 * Run multi-input prediction on a CoreML model.
 *
 * @param model           Handle from coreml_load_model().
 * @param input_names     Array of input feature names.
 * @param input_datas     Array of pointers to float32 arrays.
 * @param input_shapes    Array of shape arrays (each is int64_t*).
 * @param input_ndims_arr Array of ndim counts, one per input.
 * @param num_inputs      Number of inputs.
 * @param output_name     Output feature name.
 * @param output_data     Pre-allocated float32 output buffer.
 * @param output_size     Size of output buffer in number of floats.
 * @param error_out       If non-NULL, receives error string on failure.
 * @return 0 on success, -1 on failure.
 */
int coreml_predict_multi(coreml_model_t model,
                         const char** input_names,
                         const float** input_datas,
                         const int64_t** input_shapes,
                         const int* input_ndims_arr,
                         int num_inputs,
                         const char* output_name,
                         float* output_data,
                         int64_t output_size,
                         char** error_out);

/**
 * Free a loaded CoreML model.
 * Safe to call with NULL.
 */
void coreml_free_model(coreml_model_t model);

/**
 * Free an error string returned via error_out parameters.
 * Safe to call with NULL.
 */
void coreml_free_string(char* str);

#ifdef __cplusplus
}
#endif

#endif /* COREML_BRIDGE_H */
