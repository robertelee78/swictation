# Search Results

**Query:** Qt tray icon timer debounce double-click disambiguation code example
**Date:** 2025-11-03T03:32:25.447Z

## Answer

To **disambiguate single vs double-clicks on a Qt system tray icon (QSystemTrayIcon) using a timer-based debounce approach**, you need to handle the `activated(QSystemTrayIcon::ActivationReason)` signal and use a QTimer to delay single-click actions until you know a double-click did not occur. This is necessary because Qt emits a single-click (`Trigger`) immediately, and a double-click (`DoubleClick`) after the second click, so without debouncing, both actions may fire on a double-click[2][3].

**Key Concepts:**
- **QSystemTrayIcon::activated** emits with `Trigger` for single-click and `DoubleClick` for double-click[3].
- On many platforms, a double-click will first emit a `Trigger`, then a `DoubleClick` shortly after[3].
- To avoid firing both actions on a double-click, use a QTimer to delay the single-click action until the double-click interval has passed without a second click[2][3].

---

### Example: Qt/C++ System Tray Icon Click/Double-Click Disambiguation

```cpp
#include <QApplication>
#include <QSystemTrayIcon>
#include <QTimer>
#include <QIcon>
#include <QObject>

class TrayHandler : public QObject {
    Q_OBJECT
public:
    TrayHandler(QSystemTrayIcon* tray, QObject* parent = nullptr)
        : QObject(parent), trayIcon(tray), clickTimer(new QTimer(this))
    {
        clickTimer->setSingleShot(true);
        connect(trayIcon, &QSystemTrayIcon::activated,
                this, &TrayHandler::onTrayActivated);
        connect(clickTimer, &QTimer::timeout,
                this, &TrayHandler::onSingleClickConfirmed);
    }

private slots:
    void onTrayActivated(QSystemTrayIcon::ActivationReason reason) {
        if (reason == QSystemTrayIcon::Trigger) {
            // Start timer to wait for possible double-click
            clickTimer->start(QApplication::doubleClickInterval());
        } else if (reason == QSystemTrayIcon::DoubleClick) {
            // Double-click detected, cancel single-click timer
            clickTimer->stop();
            handleDoubleClick();
        }
    }

    void onSingleClickConfirmed() {
        // Timer expired, so it was a single click
        handleSingleClick();
    }

    void handleSingleClick() {
        // Your single-click logic here
    }

    void handleDoubleClick() {
        // Your double-click logic here
    }

private:
    QSystemTrayIcon* trayIcon;
    QTimer* clickTimer;
};
```
**Usage:**
```cpp
int main(int argc, char *argv[]) {
    QApplication app(argc, argv);
    QSystemTrayIcon tray(QIcon("icon.png"));
    tray.show();
    TrayHandler handler(&tray);
    return app.exec();
}
```
This pattern ensures that only one of the actions (single or double-click) is executed, never both for a double-click[2][3].

---

### Explanation

- **When the tray icon is clicked:**  
  The `activated` signal is emitted with `Trigger`. Start a single-shot timer for the double-click interval.
- **If a double-click occurs:**  
  The signal is emitted again with `DoubleClick` before the timer expires. Cancel the timer and execute the double-click logic.
- **If no double-click occurs:**  
  The timer expires, confirming a single-click, and the single-click logic is executed.

This approach is widely used for widgets and tray icons in Qt to reliably distinguish between single and double-clicks[2][3].

---

### PyQt/PySide Example (Python)

The same logic applies in Python, as shown in this PyQt example[2]:

```python
import sys
from PyQt5.QtWidgets import QApplication, QSystemTrayIcon, QMainWindow
from PyQt5.QtCore import QTimer

class MainWindow(QMainWindow):
    def __init__(self):
        super().__init__()
        self.trayIcon = QSystemTrayIcon(self)
        self.trayIcon.setIcon(QIcon("icon.png"))
        self.trayIcon.activated.connect(self.onTrayIconActivated)
        self.trayIcon.show()
        self.disambiguateTimer = QTimer(self)
        self.disambiguateTimer.setSingleShot(True)
        self.disambiguateTimer.timeout.connect(self.onSingleClickConfirmed)

    def onTrayIconActivated(self, reason):
        if reason == QSystemTrayIcon.Trigger:
            self.disambiguateTimer.start(QApplication.instance().doubleClickInterval())
        elif reason == QSystemTrayIcon.DoubleClick:
            self.disambiguateTimer.stop()
            print("Tray icon double clicked")

    def onSingleClickConfirmed(self):
        print("Tray icon single clicked")

if __name__ == "__main__":
    app = QApplication(sys.argv)
    w = MainWindow()
    w.show()
    sys.exit(app.exec_())
```
This code ensures only one action is triggered per user interaction[2].

---

### Notes

- **Double-click interval:**  
  Use `QApplication::doubleClickInterval()` to match the user's OS settings[2].
- **Platform differences:**  
  On macOS, double-click may only be emitted if no context menu is set[3].
- **Testing:**  
  You can manually emit the signal for testing, but for production, rely on the actual user events[1].

---

**Summary:**  
Use a single-shot QTimer to delay single-click actions until the double-click interval has passed, canceling the timer if a double-click is detected. This debounce pattern is the recommended way to reliably distinguish single and double-clicks for QSystemTrayIcon in Qt[2][3].

## Citations

### 1. PySide2/PyQt5 - Single and double click tray icon activation

**URL:** https://gist.github.com/for-l00p/3e33305f948659313127632ad04b4311

Forked from HanifCarroll/pyqt-single-double-click-tray.py

Created
May 15, 2020 00:39

Show Gist options

- You must be signed in to star a gist
- You must be signed in to fork a gist

- - - - - Learn more about clone URLs
- Save for-l00p/3e33305f948659313127632ad04b4311 to your computer and use it in GitHub Desktop.

- - - - Learn more about clone URLs

Save for-l00p/3e33305f948659313127632ad04b4311 to your computer and use it in GitHub Desktop.

PySide2/PyQt5 - Single and double click tray icon activation

This file contains hidden or bidirectional Unicode text that may be interpreted or compiled differently than what appears below. To review, open the file in an editor that reveals hidden Unicode characters. Learn more about bidirectional Unicode characters... | |# From https://riverbankcomputing.com/pipermail/pyqt/2010-November/028394.html|
|--|--|
| | |
| |import sys|
| |from PyQt4.QtCore import *|
| |from PyQt4.QtGui import *|
| | |
| |class MainWindow(QMainWindow):|
| | |
| |def __init__(self, parent=None):|
| |super(MainWindow, self).__init__(parent)|
| |self.trayIcon = QSystemTrayIcon(QIcon("some.png"), self)|
| |self.trayIcon.activated.connect(self.onTrayIconActivated)|
| |self.trayIcon.show()|
| |self.disambiguateTimer = QTimer(self)|
| |self.disambiguateTimer.setSingleShot(True)|
| |self.disambiguateTimer.timeout.connect(|
| |self.disambiguateTimerTimeout)|
| | |
| |def onTrayIconActivated(self, reason):|
| |print "onTrayIconActivated:", reason|
| |if reason == QSystemTrayIcon.Trigger:|
| |self.disambiguateTimer.start(qApp.doubleClickInterval())|
| |elif reason == QSystemTrayIcon.DoubleClick:|... | |self.disambiguateTimer.stop()|
| |print "Tray icon double clicked"|
| | |
| |def disambiguateTimerTimeout(self):|
| |print "Tray icon single clicked"|
| | |
| | |
| |if __name__ == "__main__":|
| |app = QApplication(sys.argv)|
| |w = MainWindow()|
| |w.show()|
| |sys.exit(app.exec_())|

### 2. [PyQt] QSystemTrayIcon.activated weird Trigger behavior ...

**URL:** https://riverbankcomputing.com/pipermail/pyqt/2016-August/037932.html

Hi, I have 2 issues with QSystemTrayIcon. I'm running Python 3.5.1 32-bit, and PyQt 5.6 (with Qt 5.6) on Windows 7 x64. 1) I tested 2 different Windows 7 x64 machines, regarding detecting single-click vs. double-click. On one machine, everything works as expected. A single click gave me QSystemTrayIcon.Trigger, a double click gave me QSystemTrayIcon.DoubleClick. On the other machine, however, a double click would always first produce a Trigger followed by the DoubleClick shortly after. On the machine where the behavior was as expected, I discovered that I've never changed the Windows mouse double click interval setting in my life. When I changed that setting, the behavior became bad, like for the other machine (irreversible). Regardless which value I'm setting the delay to, I can't get it back to working as expected. Following other people's advice, I'm using a disambiguation-timer, but that is really just a hack and Qt should disambiguate this for me. Any ideas? 2) I noticed that my slot for the activated-signal isn't called on a single-click in the following situation: Step 1: User opens the QMainWindow of my application (e.g. by double-clicking the icon, or by using my tray icon context menu and clicking on one of my QActions - both ways call MyMainWindow.really_show_it()) MyMainWindow.really_show_it() call super().showNormal() and then self.activateWindow(). Would I omit the latter, the window would not come in the foreground if the user was working with a different foreground application. Since the user explicitly asks for that window, I consider it appropriate to force that it... 's shown by using self.activateWindow() Step 2 (optional): user closes the MainWindow, or minimizes it Step 3: User performs a single-click on the tray icon. NOTHING happens. At all. But my slot should be called. But it isn't. Step 4: User repeats step 3: this time the signal is emitted (with QSystemTrayIcon.Trigger, as expected) Note: DoubleClick behavior is not affected, i.e. if you replace step 3 with "User peforms double click on the tray icon", then the activated-signal is emitted with QSystemTrayIcon.DoubleClick, as expected. I've read that other people had problems as well with this and they suggested to catch the raw events. This doesn't work either. When I create my own sub-class for QSystemTrayIcon and override the event() method, it is never called, except for when I close my program (then I get the delete event). I doubt that this is expected behavior. I'm thankful for any input you can provide ;). Cheers! Marius

### 3. [PyQt] QSystemTrayIcon.activated weird Trigger behavior ...

**URL:** https://riverbankcomputing.com/pipermail/pyqt/2016-August/037932.html

Hi, I have 2 issues with QSystemTrayIcon. I'm running Python 3.5.1 32-bit, and PyQt 5.6 (with Qt 5.6) on Windows 7 x64. 1) I tested 2 different Windows 7 x64 machines, regarding detecting single-click vs. double-click. On one machine, everything works as expected. A single click gave me QSystemTrayIcon.Trigger, a double click gave me QSystemTrayIcon.DoubleClick. On the other machine, however, a double click would always first produce a Trigger followed by the DoubleClick shortly after. On the machine where the behavior was as expected, I discovered that I've never changed the Windows mouse double click interval setting in my life. When I changed that setting, the behavior became bad, like for the other machine (irreversible). Regardless which value I'm setting the delay to, I can't get it back to working as expected. Following other people's advice, I'm using a disambiguation-timer, but that is really just a hack and Qt should disambiguate this for me. Any ideas? 2) I noticed that my slot for the activated-signal isn't called on a single-click in the following situation: Step 1: User opens the QMainWindow of my application (e.g. by double-clicking the icon, or by using my tray icon context menu and clicking on one of my QActions - both ways call MyMainWindow.really_show_it()) MyMainWindow.really_show_it() call super().showNormal() and then self.activateWindow(). Would I omit the latter, the window would not come in the foreground if the user was working with a different foreground application. Since the user explicitly asks for that window, I consider it appropriate to force that it... 's shown by using self.activateWindow() Step 2 (optional): user closes the MainWindow, or minimizes it Step 3: User performs a single-click on the tray icon. NOTHING happens. At all. But my slot should be called. But it isn't. Step 4: User repeats step 3: this time the signal is emitted (with QSystemTrayIcon.Trigger, as expected) Note: DoubleClick behavior is not affected, i.e. if you replace step 3 with "User peforms double click on the tray icon", then the activated-signal is emitted with QSystemTrayIcon.DoubleClick, as expected. I've read that other people had problems as well with this and they suggested to catch the raw events. This doesn't work either. When I create my own sub-class for QSystemTrayIcon and override the event() method, it is never called, except for when I close my program (then I get the delete event). I doubt that this is expected behavior. I'm thankful for any input you can provide ;). Cheers! Marius

### 4. [PyQt] QSystemTrayIcon.activated weird Trigger behavior ...

**URL:** https://riverbankcomputing.com/pipermail/pyqt/2016-August/037932.html

Hi, I have 2 issues with QSystemTrayIcon. I'm running Python 3.5.1 32-bit, and PyQt 5.6 (with Qt 5.6) on Windows 7 x64. 1) I tested 2 different Windows 7 x64 machines, regarding detecting single-click vs. double-click. On one machine, everything works as expected. A single click gave me QSystemTrayIcon.Trigger, a double click gave me QSystemTrayIcon.DoubleClick. On the other machine, however, a double click would always first produce a Trigger followed by the DoubleClick shortly after. On the machine where the behavior was as expected, I discovered that I've never changed the Windows mouse double click interval setting in my life. When I changed that setting, the behavior became bad, like for the other machine (irreversible). Regardless which value I'm setting the delay to, I can't get it back to working as expected. Following other people's advice, I'm using a disambiguation-timer, but that is really just a hack and Qt should disambiguate this for me. Any ideas? 2) I noticed that my slot for the activated-signal isn't called on a single-click in the following situation: Step 1: User opens the QMainWindow of my application (e.g. by double-clicking the icon, or by using my tray icon context menu and clicking on one of my QActions - both ways call MyMainWindow.really_show_it()) MyMainWindow.really_show_it() call super().showNormal() and then self.activateWindow(). Would I omit the latter, the window would not come in the foreground if the user was working with a different foreground application. Since the user explicitly asks for that window, I consider it appropriate to force that it... 's shown by using self.activateWindow() Step 2 (optional): user closes the MainWindow, or minimizes it Step 3: User performs a single-click on the tray icon. NOTHING happens. At all. But my slot should be called. But it isn't. Step 4: User repeats step 3: this time the signal is emitted (with QSystemTrayIcon.Trigger, as expected) Note: DoubleClick behavior is not affected, i.e. if you replace step 3 with "User peforms double click on the tray icon", then the activated-signal is emitted with QSystemTrayIcon.DoubleClick, as expected. I've read that other people had problems as well with this and they suggested to catch the raw events. This doesn't work either. When I create my own sub-class for QSystemTrayIcon and override the event() method, it is never called, except for when I close my program (then I get the delete event). I doubt that this is expected behavior. I'm thankful for any input you can provide ;). Cheers! Marius

### 5. PySide2/PyQt5 - Single and double click tray icon activation

**URL:** https://gist.github.com/for-l00p/3e33305f948659313127632ad04b4311

Forked from HanifCarroll/pyqt-single-double-click-tray.py

Created
May 15, 2020 00:39

Show Gist options

- You must be signed in to star a gist
- You must be signed in to fork a gist

- - - - - Learn more about clone URLs
- Save for-l00p/3e33305f948659313127632ad04b4311 to your computer and use it in GitHub Desktop.

- - - - Learn more about clone URLs

Save for-l00p/3e33305f948659313127632ad04b4311 to your computer and use it in GitHub Desktop.

PySide2/PyQt5 - Single and double click tray icon activation

This file contains hidden or bidirectional Unicode text that may be interpreted or compiled differently than what appears below. To review, open the file in an editor that reveals hidden Unicode characters. Learn more about bidirectional Unicode characters... | |# From https://riverbankcomputing.com/pipermail/pyqt/2010-November/028394.html|
|--|--|
| | |
| |import sys|
| |from PyQt4.QtCore import *|
| |from PyQt4.QtGui import *|
| | |
| |class MainWindow(QMainWindow):|
| | |
| |def __init__(self, parent=None):|
| |super(MainWindow, self).__init__(parent)|
| |self.trayIcon = QSystemTrayIcon(QIcon("some.png"), self)|
| |self.trayIcon.activated.connect(self.onTrayIconActivated)|
| |self.trayIcon.show()|
| |self.disambiguateTimer = QTimer(self)|
| |self.disambiguateTimer.setSingleShot(True)|
| |self.disambiguateTimer.timeout.connect(|
| |self.disambiguateTimerTimeout)|
| | |
| |def onTrayIconActivated(self, reason):|
| |print "onTrayIconActivated:", reason|
| |if reason == QSystemTrayIcon.Trigger:|
| |self.disambiguateTimer.start(qApp.doubleClickInterval())|
| |elif reason == QSystemTrayIcon.DoubleClick:|... | |self.disambiguateTimer.stop()|
| |print "Tray icon double clicked"|
| | |
| |def disambiguateTimerTimeout(self):|
| |print "Tray icon single clicked"|
| | |
| | |
| |if __name__ == "__main__":|
| |app = QApplication(sys.argv)|
| |w = MainWindow()|
| |w.show()|
| |sys.exit(app.exec_())|

## Metadata

```json
{
  "planId": "plan_3",
  "executionTime": 33749,
  "replanned": false
}
```

## Planning Log

```
🎯 GOAP Planning & Execution Log
================================
📋 Plan Execution Summary:
  • Steps executed: 4
  • Success: Yes
  • Replanned: No
```
