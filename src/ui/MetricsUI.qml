import QtQuick 2.15
import QtQuick.Window 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15

Window {
    id: root
    width: 800
    height: 600
    title: "Swictation Metrics"
    visible: false

    // Tokyo Night Dark theme colors
    color: "#1a1b26"

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 20
        spacing: 10

        // Header
        Text {
            text: "Swictation Metrics Dashboard"
            font.pixelSize: 24
            font.bold: true
            color: "#c0caf5"
            Layout.alignment: Qt.AlignHCenter
        }

        // Connection status
        RowLayout {
            Layout.fillWidth: true
            spacing: 10

            Rectangle {
                width: 12
                height: 12
                radius: 6
                color: backend.connected ? "#9ece6a" : "#f7768e"
            }

            Text {
                text: backend.connected ? "Connected to daemon" : "Disconnected"
                color: "#a9b1d6"
                font.pixelSize: 14
            }
        }

        // Live metrics section
        GroupBox {
            Layout.fillWidth: true
            title: "Live Session"

            background: Rectangle {
                color: "#24283b"
                border.color: "#414868"
                radius: 8
            }

            label: Text {
                text: "Live Session"
                color: "#c0caf5"
                font.pixelSize: 16
                font.bold: true
            }

            GridLayout {
                columns: 2
                columnSpacing: 20
                rowSpacing: 10

                Text { text: "State:"; color: "#7aa2f7"; font.pixelSize: 14 }
                Text { text: backend.state; color: "#c0caf5"; font.pixelSize: 14; font.bold: true }

                Text { text: "WPM:"; color: "#7aa2f7"; font.pixelSize: 14 }
                Text { text: backend.wpm.toFixed(1); color: "#c0caf5"; font.pixelSize: 14; font.bold: true }

                Text { text: "Words:"; color: "#7aa2f7"; font.pixelSize: 14 }
                Text { text: backend.words; color: "#c0caf5"; font.pixelSize: 14; font.bold: true }

                Text { text: "Latency:"; color: "#7aa2f7"; font.pixelSize: 14 }
                Text { text: backend.latency_ms.toFixed(1) + " ms"; color: "#c0caf5"; font.pixelSize: 14; font.bold: true }

                Text { text: "Segments:"; color: "#7aa2f7"; font.pixelSize: 14 }
                Text { text: backend.segments; color: "#c0caf5"; font.pixelSize: 14; font.bold: true }

                Text { text: "Duration:"; color: "#7aa2f7"; font.pixelSize: 14 }
                Text { text: backend.duration; color: "#c0caf5"; font.pixelSize: 14; font.bold: true }

                Text { text: "GPU Memory:"; color: "#7aa2f7"; font.pixelSize: 14 }
                Text { text: backend.gpu_memory_mb.toFixed(1) + " MB"; color: "#c0caf5"; font.pixelSize: 14; font.bold: true }

                Text { text: "CPU:"; color: "#7aa2f7"; font.pixelSize: 14 }
                Text { text: backend.cpu_percent.toFixed(1) + "%"; color: "#c0caf5"; font.pixelSize: 14; font.bold: true }
            }
        }

        // Placeholder for tabs (Phase 3 will expand this)
        Text {
            text: "Full UI with tabs coming in Phase 3..."
            color: "#565f89"
            font.pixelSize: 12
            Layout.alignment: Qt.AlignHCenter
        }

        Item { Layout.fillHeight: true }
    }
}
