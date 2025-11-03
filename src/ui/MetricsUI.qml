import QtQuick 2.15
import QtQuick.Window 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15

Window {
    id: root
    width: 1000
    height: 700
    title: "Swictation Metrics"
    visible: false

    // Tokyo Night Dark theme colors
    color: "#1a1b26"

    // Connection status indicator (top-right)
    Rectangle {
        anchors.top: parent.top
        anchors.right: parent.right
        anchors.margins: 10
        width: 100
        height: 30
        color: backend.connected ? "#9ece6a" : "#f7768e"
        radius: 4
        z: 100

        Text {
            anchors.centerIn: parent
            text: backend.connected ? "● LIVE" : "● OFFLINE"
            color: "#ffffff"
            font.pixelSize: 12
            font.bold: true
        }
    }

    // Tab bar at top
    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        TabBar {
            id: tabBar
            Layout.fillWidth: true
            background: Rectangle { color: "#24283b" }

            TabButton {
                text: "Live Session"
                font.family: "Monospace"
                font.pixelSize: 14

                contentItem: Text {
                    text: parent.text
                    font: parent.font
                    color: parent.checked ? "#7aa2f7" : "#565f89"
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                }

                background: Rectangle {
                    color: parent.checked ? "#1a1b26" : "#24283b"
                }
            }
            TabButton {
                text: "History"
                font.family: "Monospace"
                font.pixelSize: 14

                contentItem: Text {
                    text: parent.text
                    font: parent.font
                    color: parent.checked ? "#7aa2f7" : "#565f89"
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                }

                background: Rectangle {
                    color: parent.checked ? "#1a1b26" : "#24283b"
                }
            }
            TabButton {
                text: "Transcriptions"
                font.family: "Monospace"
                font.pixelSize: 14

                contentItem: Text {
                    text: parent.text
                    font: parent.font
                    color: parent.checked ? "#7aa2f7" : "#565f89"
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                }

                background: Rectangle {
                    color: parent.checked ? "#1a1b26" : "#24283b"
                }
            }
        }

        StackLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            currentIndex: tabBar.currentIndex

            // ============================================================
            // TAB 1: LIVE SESSION
            // ============================================================
            Item {
                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: 20
                    spacing: 20

                    // Status header
                    Rectangle {
                        Layout.fillWidth: true
                        height: 80
                        color: "#24283b"
                        radius: 8

                        RowLayout {
                            anchors.centerIn: parent
                            spacing: 15

                            Text {
                                text: backend.state === "recording" ? "🔴" :
                                      backend.state === "processing" ? "🟡" : "🎤"
                                font.pixelSize: 48
                            }

                            Text {
                                text: backend.state.toUpperCase()
                                font.pixelSize: 32
                                font.family: "Monospace"
                                font.bold: true
                                color: backend.state === "recording" ? "#ff5555" :
                                       backend.state === "processing" ? "#ffcc00" : "#9ece6a"
                            }
                        }
                    }

                    // Metric cards grid (3 columns)
                    GridLayout {
                        Layout.fillWidth: true
                        columns: 3
                        rowSpacing: 15
                        columnSpacing: 15

                        // WPM Card
                        Rectangle {
                            height: 100
                            color: "#24283b"
                            radius: 8
                            border.color: "#414868"
                            border.width: 1
                            Layout.fillWidth: true
                            ColumnLayout {
                                anchors.centerIn: parent
                                spacing: 5
                                Text { text: "WPM"; color: "#565f89"; font.pixelSize: 12; Layout.alignment: Qt.AlignHCenter }
                                Text { text: Math.round(backend.wpm); color: "#a9b1d6"; font.pixelSize: 28; font.bold: true; font.family: "Monospace"; Layout.alignment: Qt.AlignHCenter }
                            }
                        }
                        // Words Card
                        Rectangle {
                            height: 100
                            color: "#24283b"
                            radius: 8
                            border.color: "#414868"
                            border.width: 1
                            Layout.fillWidth: true
                            ColumnLayout {
                                anchors.centerIn: parent
                                spacing: 5
                                Text { text: "Words"; color: "#565f89"; font.pixelSize: 12; Layout.alignment: Qt.AlignHCenter }
                                Text { text: backend.words; color: "#a9b1d6"; font.pixelSize: 28; font.bold: true; font.family: "Monospace"; Layout.alignment: Qt.AlignHCenter }
                            }
                        }
                        // Latency Card
                        Rectangle {
                            height: 100
                            color: "#24283b"
                            radius: 8
                            border.color: "#414868"
                            border.width: 1
                            Layout.fillWidth: true
                            ColumnLayout {
                                anchors.centerIn: parent
                                spacing: 5
                                Text { text: "Latency"; color: "#565f89"; font.pixelSize: 12; Layout.alignment: Qt.AlignHCenter }
                                Text { text: (backend.latency_ms / 1000).toFixed(2) + "s"; color: "#a9b1d6"; font.pixelSize: 28; font.bold: true; font.family: "Monospace"; Layout.alignment: Qt.AlignHCenter }
                            }
                        }
                        // Duration Card
                        Rectangle {
                            height: 100
                            color: "#24283b"
                            radius: 8
                            border.color: "#414868"
                            border.width: 1
                            Layout.fillWidth: true
                            ColumnLayout {
                                anchors.centerIn: parent
                                spacing: 5
                                Text { text: "Duration"; color: "#565f89"; font.pixelSize: 12; Layout.alignment: Qt.AlignHCenter }
                                Text { text: backend.duration; color: "#a9b1d6"; font.pixelSize: 28; font.bold: true; font.family: "Monospace"; Layout.alignment: Qt.AlignHCenter }
                            }
                        }
                        // Segments Card
                        Rectangle {
                            height: 100
                            color: "#24283b"
                            radius: 8
                            border.color: "#414868"
                            border.width: 1
                            Layout.fillWidth: true
                            ColumnLayout {
                                anchors.centerIn: parent
                                spacing: 5
                                Text { text: "Segments"; color: "#565f89"; font.pixelSize: 12; Layout.alignment: Qt.AlignHCenter }
                                Text { text: backend.segments; color: "#a9b1d6"; font.pixelSize: 28; font.bold: true; font.family: "Monospace"; Layout.alignment: Qt.AlignHCenter }
                            }
                        }
                        // GPU Memory Card
                        Rectangle {
                            height: 100
                            color: "#24283b"
                            radius: 8
                            border.color: "#414868"
                            border.width: 1
                            Layout.fillWidth: true
                            ColumnLayout {
                                anchors.centerIn: parent
                                spacing: 5
                                Text { text: "GPU Memory"; color: "#565f89"; font.pixelSize: 12; Layout.alignment: Qt.AlignHCenter }
                                Text { text: (backend.gpu_memory_mb / 1024).toFixed(1) + " GB"; color: "#a9b1d6"; font.pixelSize: 28; font.bold: true; font.family: "Monospace"; Layout.alignment: Qt.AlignHCenter }
                            }
                        }
                    }

                    // System resources
                    Rectangle {
                        Layout.fillWidth: true
                        Layout.preferredHeight: 150
                        color: "#24283b"
                        radius: 8

                        ColumnLayout {
                            anchors.fill: parent
                            anchors.margins: 15
                            spacing: 15

                            Text {
                                text: "System Resources"
                                color: "#7aa2f7"
                                font.pixelSize: 16
                                font.bold: true
                            }

                            // GPU Memory Meter
                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 5
                                Text {
                                    text: "GPU Memory: " + backend.gpu_memory_mb.toFixed(1) + " / 8000.0 MB (" + Math.round(backend.gpu_memory_mb/8000*100) + "%)"
                                    color: "#a9b1d6"
                                    font.pixelSize: 12
                                }
                                ProgressBar {
                                    Layout.fillWidth: true
                                    value: Math.min(backend.gpu_memory_mb / 8000, 1.0)
                                    background: Rectangle { color: "#414868"; radius: 3 }
                                    contentItem: Item {
                                        Rectangle {
                                            width: parent.parent.visualPosition * parent.width
                                            height: parent.height
                                            radius: 3
                                            color: parent.parent.value > 0.8 ? "#f7768e" : parent.parent.value > 0.6 ? "#e0af68" : "#9ece6a"
                                        }
                                    }
                                }
                            }
                            // CPU Usage Meter
                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 5
                                Text {
                                    text: "CPU Usage: " + backend.cpu_percent.toFixed(1) + " / 100.0 % (" + Math.round(backend.cpu_percent/100*100) + "%)"
                                    color: "#a9b1d6"
                                    font.pixelSize: 12
                                }
                                ProgressBar {
                                    Layout.fillWidth: true
                                    value: Math.min(backend.cpu_percent / 100, 1.0)
                                    background: Rectangle { color: "#414868"; radius: 3 }
                                    contentItem: Item {
                                        Rectangle {
                                            width: parent.parent.visualPosition * parent.width
                                            height: parent.height
                                            radius: 3
                                            color: parent.parent.value > 0.8 ? "#f7768e" : parent.parent.value > 0.6 ? "#e0af68" : "#9ece6a"
                                        }
                                    }
                                }
                            }
                        }
                    }

                    Item { Layout.fillHeight: true }
                }
            }

            // ============================================================
            // TAB 2: HISTORY
            // ============================================================
            Item {
                // Store lifetime stats (must be defined BEFORE use)
                property var lifetimeStats: ({})

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: 20
                    spacing: 15

                    RowLayout {
                        Layout.fillWidth: true

                        Text {
                            text: "Recent Sessions (Last 10)"
                            color: "#a9b1d6"
                            font.pixelSize: 18
                            font.bold: true
                            Layout.fillWidth: true
                        }

                        Button {
                            text: "🔄 Refresh"
                            onClicked: {
                                historyListModel.clear()
                                let sessions = backend.loadHistory()
                                for (let i = 0; i < sessions.length; i++) {
                                    historyListModel.append(sessions[i])
                                }
                                lifetimeStats = backend.loadLifetimeStats()
                            }

                            contentItem: Text {
                                text: parent.text
                                font.pixelSize: 12
                                color: "#a9b1d6"
                                horizontalAlignment: Text.AlignHCenter
                                verticalAlignment: Text.AlignVCenter
                            }

                            background: Rectangle {
                                color: parent.hovered ? "#414868" : "#24283b"
                                radius: 4
                                border.color: "#7aa2f7"
                                border.width: 1
                            }
                        }
                    }

                    ListView {
                        id: historyList
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        model: ListModel { id: historyListModel }
                        spacing: 10
                        clip: true

                        delegate: Rectangle {
                            width: ListView.view.width
                            height: 80
                            color: "#24283b"
                            radius: 4
                            border.color: "#414868"
                            border.width: 1

                            RowLayout {
                                anchors.fill: parent
                                anchors.margins: 15
                                spacing: 20

                                Text {
                                    text: "#" + (model.id || 0)
                                    color: "#7aa2f7"
                                    font.bold: true
                                    font.pixelSize: 16
                                }

                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: 5

                                    Text {
                                        text: new Date((model.start_time || 0) * 1000).toLocaleString()
                                        color: "#a9b1d6"
                                        font.pixelSize: 12
                                    }

                                    RowLayout {
                                        spacing: 15
                                        Text { text: (model.words_dictated || 0) + " words"; color: "#9ece6a"; font.pixelSize: 11 }
                                        Text { text: Math.round(model.wpm || 0) + " WPM"; color: "#7aa2f7"; font.pixelSize: 11 }
                                        Text { text: (model.avg_latency_ms || 0).toFixed(0) + "ms"; color: "#e0af68"; font.pixelSize: 11 }
                                    }
                                }

                                Text {
                                    text: ((model.duration_s || 0) / 60).toFixed(1) + "m"
                                    color: "#565f89"
                                    font.pixelSize: 14
                                }
                            }
                        }
                    }

                    // Lifetime stats
                    Rectangle {
                        Layout.fillWidth: true
                        Layout.preferredHeight: 180
                        color: "#24283b"
                        radius: 8

                        ColumnLayout {
                            anchors.fill: parent
                            anchors.margins: 15
                            spacing: 10

                            Text {
                                text: "Lifetime Stats"
                                color: "#7aa2f7"
                                font.pixelSize: 16
                                font.bold: true
                            }

                            GridLayout {
                                Layout.fillWidth: true
                                columns: 2
                                columnSpacing: 30
                                rowSpacing: 8

                                RowLayout {
                                    spacing: 10
                                    Text { text: "Total Words:"; color: "#565f89"; font.pixelSize: 13; Layout.minimumWidth: 140 }
                                    Text { text: lifetimeStats.total_words || 0; color: "#a9b1d6"; font.bold: true; font.pixelSize: 13 }
                                }
                                RowLayout {
                                    spacing: 10
                                    Text { text: "Total Sessions:"; color: "#565f89"; font.pixelSize: 13; Layout.minimumWidth: 140 }
                                    Text { text: lifetimeStats.total_sessions || 0; color: "#a9b1d6"; font.bold: true; font.pixelSize: 13 }
                                }
                                RowLayout {
                                    spacing: 10
                                    Text { text: "Avg WPM:"; color: "#565f89"; font.pixelSize: 13; Layout.minimumWidth: 140 }
                                    Text { text: Math.round(lifetimeStats.avg_wpm || 0); color: "#a9b1d6"; font.bold: true; font.pixelSize: 13 }
                                }
                                RowLayout {
                                    spacing: 10
                                    Text { text: "Time Saved:"; color: "#565f89"; font.pixelSize: 13; Layout.minimumWidth: 140 }
                                    Text { text: ((lifetimeStats.time_saved_minutes || 0) / 60).toFixed(1) + "h"; color: "#a9b1d6"; font.bold: true; font.pixelSize: 13 }
                                }
                                RowLayout {
                                    spacing: 10
                                    Text { text: "Best WPM:"; color: "#565f89"; font.pixelSize: 13; Layout.minimumWidth: 140 }
                                    Text { text: Math.round(lifetimeStats.best_wpm_value || 0); color: "#a9b1d6"; font.bold: true; font.pixelSize: 13 }
                                }
                                RowLayout {
                                    spacing: 10
                                    Text { text: "Lowest Latency:"; color: "#565f89"; font.pixelSize: 13; Layout.minimumWidth: 140 }
                                    Text { text: (lifetimeStats.lowest_latency_ms || 0).toFixed(0) + "ms"; color: "#a9b1d6"; font.bold: true; font.pixelSize: 13 }
                                }
                            }
                        }
                    }
                }

                // Load data when tab is activated
                Component.onCompleted: {
                    historyListModel.clear()
                    let sessions = backend.loadHistory()
                    for (let i = 0; i < sessions.length; i++) {
                        historyListModel.append(sessions[i])
                    }
                    lifetimeStats = backend.loadLifetimeStats()
                }
            }

            // ============================================================
            // TAB 3: TRANSCRIPTIONS
            // ============================================================
            Item {
                Rectangle {
                    anchors.fill: parent
                    anchors.margins: 20
                    color: "#24283b"
                    radius: 8

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: 15
                        spacing: 10

                        Text {
                            text: "Session Transcriptions (Ephemeral)"
                            font.pixelSize: 18
                            font.bold: true
                            color: "#7aa2f7"
                        }

                        Text {
                            text: "🔒 Privacy: Not saved to disk, RAM-only"
                            font.pixelSize: 11
                            font.italic: true
                            color: "#565f89"
                        }

                        ListView {
                            id: transcriptionList
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            model: ListModel { id: transcriptionModel }
                            spacing: 10
                            clip: true

                            delegate: Rectangle {
                                width: ListView.view.width
                                height: contentColumn.height + 20
                                color: "#1a1b26"
                                radius: 4
                                border.color: "#7aa2f7"
                                border.width: 1

                                ColumnLayout {
                                    id: contentColumn
                                    anchors.fill: parent
                                    anchors.margins: 10
                                    spacing: 5

                                    RowLayout {
                                        spacing: 8

                                        Text {
                                            text: model.timestamp
                                            color: "#565f89"
                                            font.pixelSize: 11
                                            font.family: "Monospace"
                                        }
                                        Text {
                                            text: " │ "
                                            color: "#565f89"
                                        }
                                        Text {
                                            text: Math.round(model.wpm) + " WPM"
                                            color: "#565f89"
                                            font.pixelSize: 11
                                        }
                                        Text {
                                            text: " │ "
                                            color: "#565f89"
                                        }
                                        Text {
                                            text: (model.latency / 1000).toFixed(2) + "s"
                                            color: model.latency > 2000 ? "#f7768e" : "#565f89"
                                            font.pixelSize: 11
                                        }

                                        Item { Layout.fillWidth: true }
                                    }

                                    Text {
                                        text: model.text
                                        color: "#a9b1d6"
                                        font.family: "Monospace"
                                        font.pixelSize: 13
                                        wrapMode: Text.WordWrap
                                        Layout.fillWidth: true
                                    }
                                }
                            }

                            // Auto-scroll to bottom when new items added
                            onCountChanged: {
                                positionViewAtEnd()
                            }
                        }

                        Text {
                            text: "⚠️  Buffer clears when you start a new session"
                            color: "#565f89"
                            font.pixelSize: 11
                            font.italic: true
                        }
                    }
                }
            }
        }
    }

    // Connect to backend signals
    Connections {
        target: backend

        function onTranscriptionAdded(text, timestamp, wpm, latency) {
            transcriptionModel.append({
                text: text,
                timestamp: timestamp,
                wpm: wpm,
                latency: latency
            })
        }

        function onSessionCleared() {
            transcriptionModel.clear()
        }

        function onRecentSessionsChanged() {
            // Refresh history when session ends
            if (tabBar.currentIndex === 1) {  // Only if History tab is visible
                historyListModel.clear()
                let sessions = backend.loadHistory()
                for (let i = 0; i < sessions.length; i++) {
                    historyListModel.append(sessions[i])
                }
            }
        }
    }
}

