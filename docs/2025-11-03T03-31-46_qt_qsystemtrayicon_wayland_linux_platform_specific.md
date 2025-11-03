# Search Results

**Query:** Qt QSystemTrayIcon Wayland Linux platform-specific behavior differences
**Date:** 2025-11-03T03:31:46.416Z

## Answer

**QSystemTrayIcon** exhibits significant platform-specific behavior differences on Linux, especially when comparing traditional X11 environments to modern Wayland-based desktops. The following details the key differences and challenges:

---

### **1. X11 vs. Wayland: System Tray Support**

- **X11:**  
  On X11, QSystemTrayIcon relies on the [freedesktop.org XEmbed system tray specification](https://standards.freedesktop.org/systemtray-spec/systemtray-spec-0.2.html), which is widely supported by traditional Linux desktop environments (KDE, older GNOME, Xfce, LXQt, etc.)[4][5].  
  - The tray icon is managed by the window manager or a dedicated system tray applet.
  - Features like context menus, tooltips, and activation signals are generally supported as expected[5].

- **Wayland:**  
  Wayland does **not** natively support the legacy XEmbed system tray protocol. Instead, it encourages the use of the [StatusNotifierItem (SNI) specification](https://www.freedesktop.org/wiki/Specifications/StatusNotifierItem/), which is a D-Bus-based protocol[2].  
  - **KDE Plasma (Wayland):** KDE has implemented a compatibility layer that maps QSystemTrayIcon to SNI, allowing tray icons to work in Plasma Wayland sessions, but this required changes in both Qt and KDE Frameworks and only became robust in Qt 5.4 and later[2].
  - **GNOME (Wayland):** GNOME Shell removed legacy tray support in version 3.26. Tray icons from QSystemTrayIcon will not appear unless the user installs third-party extensions (e.g., "AppIndicator" or "KStatusNotifierItem" extensions)[5][4].
  - **Other Environments:** Support is inconsistent; some environments may not display tray icons at all, or may display them incorrectly (e.g., as blank icons or in unexpected locations)[4].

---

### **2. Application Behavior and Limitations**

- **Detection:**  
  `QSystemTrayIcon::isSystemTrayAvailable()` may return `false` on Wayland or in environments without a compatible tray implementation, preventing the icon from being shown[1][5].

- **Feature Gaps:**  
  - **Tooltips and Wheel Events:** Only supported on X11. On Wayland, these events are not delivered to QSystemTrayIcon[5].
  - **Activation Reasons:** Not all activation reasons (e.g., middle-click, double-click) are supported on GNOME Shell without extensions[5].
  - **Notifications:** Balloon messages (`showMessage()`) may not appear or may behave differently depending on the environment and user configuration.

- **Implementation Details:**  
  - On KDE Plasma/Wayland, QSystemTrayIcon is forced to use the SNI API via the platform theme, but this only works if the Qt build and the desktop environment both support it[2].
  - On GNOME/Wayland, unless an extension is installed, tray icons will not be visible, and there is no fallback[4][8].

---

### **3. Workarounds and Compatibility Layers**

- **XWayland:**  
  Many Qt applications on Wayland still run under XWayland (an X11 compatibility layer), which allows QSystemTrayIcon to function as it does on X11, but this is not a long-term solution as distributions move toward pure Wayland sessions[3][5].

- **AppIndicator/SNI-Qt:**  
  Some distributions (notably Ubuntu) use `sni-qt` or similar shims to translate legacy tray icon calls to the SNI protocol, but this is not universal and may not be available or maintained in all environments[2][8].

- **Extensions:**  
  Users on GNOME/Wayland must install extensions to restore tray icon functionality for legacy applications[8].

---

### **4. Summary Table: QSystemTrayIcon Behavior**

| Environment                | QSystemTrayIcon Support | Notes                                                                                  |
|----------------------------|------------------------|----------------------------------------------------------------------------------------|
| KDE Plasma (X11)           | Full                   | Uses XEmbed; all features supported                                                    |
| KDE Plasma (Wayland)       | Partial/Full           | Uses SNI via platform theme; requires Qt ≥ 5.4 and KDE Frameworks integration[2]       |
| GNOME (X11)                | Partial                | Tray icons may appear, but support is incomplete or buggy in newer GNOME versions[4]   |
| GNOME (Wayland)            | None/Extension-based   | No native support; requires user-installed extensions for tray icons to appear[4][8]   |
| Xfce, LXQt, MATE (X11)     | Full                   | Uses XEmbed; tray icons work as expected                                               |
| Xfce, LXQt, MATE (Wayland) | Varies                 | Depends on compositor and SNI support; often unsupported or buggy                      |

---

### **5. Developer Recommendations**

- **Check for Tray Availability:** Always use `QSystemTrayIcon::isSystemTrayAvailable()` before creating the icon[1].
- **Target SNI:** For modern Linux desktops, prefer the StatusNotifierItem protocol (directly or via libraries like libappindicator) for maximum compatibility.
- **Inform Users:** Document any requirements for extensions or compatibility layers on Wayland-based desktops.
- **Test Across Environments:** Behavior can vary widely; test on both X11 and Wayland, and across major desktop environments.

---

**In summary:**  
**QSystemTrayIcon** works reliably on X11-based Linux desktops but faces significant limitations or outright lack of support on Wayland, especially in GNOME. KDE Plasma Wayland provides a compatibility layer, but this is not universal. Developers should be aware of these differences and consider alternative approaches for tray icon functionality on modern Linux systems[2][4][5][8].

## Citations

### 1. Some Qt applications with 'QT_QPA_PLATFORM=wayland' fail to ...

**URL:** https://bbs.archlinux.org/viewtopic.php?id=266967

Since I enjoy wayland on Gnome I was interested in enabling it for my Qt applications as well. After consulting the wiki, installing 'qt5-wayland', adding the 'QT_QPA_PLATFORM=wayland' environment variable and rebooting, some (but not all!) of my Qt applications won't start anymore. While f.e. monero-wallet-gui and qt5ct work perfectly fine, i7z-gui or onlyoffice-desktopeditors don't. First, here's the error message I'm getting:

```

This application failed to start because it could not find or load the Qt platform plugin "wayland" in "".

Available platform plugins are: linuxfb, minimal, offscreen, vnc, xcb.

```

To troubleshoot, I tried exporting the QT_PLUGIN_PATH variable ('export QT_PLUGIN_PATH=/usr/lib/qt/plugins') as recommended on some forums, but that didn't help.

Here are my relevant environment variables:

```

XDG_SESSION_TYPE=wayland

QT_STYLE_OVERRIDE=adwaita

QT_QPA_PLATFORMTHEME=gnome

QT_QPA_PLATFORM=wayland

```

Here are my relevant packages:

```

adwaita-qt 1.3.1-1

qgnomeplatform 0.8.0-2

qt5ct 1.2-1

qt5-base 5.15.2+kde+r196-1

qt5-wayland 5.15.2+kde+r23-1

wayland 1.19.0-1

wayland-protocols 1.21-1

xorg-xwayland 21.1.1-1

```... Any suggestions?

Offline

i7z-gui

wayland doesn't allow running GUI applications as root

onlyoffice-desktopeditors

No idea what that is, but sounds like a proprietary application which embeds its own Qt version, which probably doesn't include qtwayland.

Offline

wayland doesn't allow running GUI applications as root

That makes sense, thanks.

No idea what that is, but sounds like a proprietary application which embeds its own Qt version, which probably doesn't include qtwayland.

The command belongs to OnlyOffice, a free/libre office suite which strives for 100% compatibility with M$hit Office files (for the case where LibreOffice fails to format correctly). I got it as a binary from the AUR, so I guess that's why it isn't picking up my external qt5-wayland plugin (it appears that Qt is shipped bundled with the binary).

Well, I guess that explains everything really. I assume it isn't possible, but is there something I can do to keep the 'QT_QPA_PLATFORM=wayland' environment variable globally but override/unset it for certain applications, regardless of how I call them (f.e. over the command line, over my desktop environment, when some program invokes it, etc.)?

*Last edited by indignation (2021-06-07 13:37:54)*... Offline

The easiest way is probably editing the .desktop file.

Offline

arojas wrote:

wayland doesn't allow running GUI applications as root

That makes sense, thanks.

You can probably use waypipe to run the application as root and display it in your user wayland compositor, but it is not recommended to have all that GUI code running as root to minimize the attack surface.

Well, I guess that explains everything really. I assume it isn't possible, but is there something I can do to keep the 'QT_QPA_PLATFORM=wayland' environment variable globally but override/unset it for certain applications, regardless of how I call them (f.e. over the command line, over my desktop environment, when some program invokes it, etc.)?

As icar suggested, use the .desktop file. https://wiki.archlinux.org/title/Deskto … _variables

For the shell define an alias or a function. You could also override the execuable in the PATH with a script in /usr/local/bin that does the necessary setup.

*Last edited by progandy (2021-06-07 14:30:37)*



*alias CUTF='LANG=en_XX.UTF-8@POSIX '* |

Offline

### 2. qtbase/src/widgets/util/qsystemtrayicon.cpp at dev · qt/qtbase

**URL:** https://github.com/qt/qtbase/blob/dev/src/widgets/util/qsystemtrayicon.cpp

// Copyright (C) 2016 The Qt Company Ltd.
// SPDX-License-Identifier: LicenseRef-Qt-Commercial OR LGPL-3.0-only OR GPL-2.0-only OR GPL-3.0-only

#include "qsystemtrayicon.h"
#include "qsystemtrayicon_p.h"

#ifndef QT_NO_SYSTEMTRAYICON

#if QT_CONFIG(menu)
#include "qmenu.h"
#endif
#include "qlist.h"
#include "qevent.h"
#include "qpoint.h"
#if QT_CONFIG(label)
#include "qlabel.h"
#include "private/qlabel_p.h"
#endif
#if QT_CONFIG(pushbutton)
#include "qpushbutton.h"
#endif
#include "qpainterpath.h"
#include "qpainter.h"
#include "qstyle.h"
#include "qgridlayout.h"
#include "qapplication.h"
#include "qbitmap.h"

#include <private/qhighdpiscaling_p.h>
#include <qpa/qplatformscreen.h>

using namespace std::chrono_literals;

QT_BEGIN_NAMESPACE

static QIcon messageIcon2qIcon(QSystemTrayIcon::MessageIcon icon)
{
    QStyle::StandardPixmap stdIcon = QStyle::SP_CustomBase; // silence gcc 4.9.0 about uninited variable
    switch (icon) {
    case QSystemTrayIcon::Information:
        stdIcon = QStyle::SP_MessageBoxInformation;
        break;
    case QSystemTrayIcon::Warning:
        stdIcon = QStyle::SP_MessageBoxWarning;
        break;
    case QSystemTrayIcon::Critical:
        stdIcon = QStyle::SP_MessageBoxCritical;
        break;
    case QSystemTrayIcon::NoIcon:
        return QIcon();
    }
    return QApplication::style()->standardIcon(stdIcon);
}... /*!
    \class QSystemTrayIcon
    \brief The QSystemTrayIcon class provides an icon for an application in the system tray.
    \since 4.2
    \ingroup desktop
    \inmodule QtWidgets...     Modern operating systems usually provide a special area on the desktop,
    called the \e{system tray} or \e{notification area}, where long-running
    applications can display icons and short messages.

    \image system-tray.webp The system tray on Windows 10.

    The QSystemTrayIcon class can be used on the following platforms:

    \list
    \li All supported versions of Windows.
    \li All Linux desktop environments that implement the D-Bus
       \l{http://www.freedesktop.org/wiki/Specifications/StatusNotifierItem/StatusNotifierItem}
       {StatusNotifierItem specification}, including KDE, Gnome, Xfce, LXQt, and DDE.
    \li All window managers and independent tray implementations for X11 that implement the
       \l{http://standards.freedesktop.org/systemtray-spec/systemtray-spec-0.2.html}
       {freedesktop.org XEmbed system tray specification}.
    \li All supported versions of \macos.
    \endlist

    To check whether a system tray is present on the user's desktop,
    call the QSystemTrayIcon::isSystemTrayAvailable() static function.

    To add a system tray entry, create a QSystemTrayIcon object, call setContextMenu()
    to provide a context menu for the icon, and call show() to make it visible in the
    system tray. Status notification messages ("balloon messages") can be displayed at
    any time using showMessage().

    If the system tray is unavailable when a system tray icon is constructed, but
    becomes available later, QSystemTrayIcon will automatically add an entry for the
    application in the system tray if the icon is \l visible.

    The activated() signal is emitted when the user activates the icon.

    Only on X11, when a tooltip is requested, the QSystemTrayIcon receives a QHelpEvent
    of type QEvent::ToolTip. Additionally, the QSystemTrayIcon receives wheel events of
    type QEvent::Wheel. These are not supported on any other platform. Note: Since GNOME
    Shell version 3.26, not all QSystemTrayIcon::ActivationReason are supported by the
    system without shell extensions installed.

    \sa QDesktopServices, {Desktop Integration}, {System Tray Icon Example}... */

/*!
    \enum QSystemTrayIcon::MessageIcon

    This enum describes the icon that is shown when a balloon message is displayed.

    \value NoIcon No icon is shown.
    \value Information An information icon is shown.
    \value Warning A standard warning icon is shown.
    \value Critical A critical warning icon is shown.

    \sa QMessageBox
*/

/*!
    Constructs a QSystemTrayIcon object with the given \a parent.

    The icon is initially invisible.

    \sa visible
*/
QSystemTrayIcon::QSystemTrayIcon(QObject *parent)
: QObject(*new QSystemTrayIconPrivate(), parent)
{
}

/*!
    Constructs a QSystemTrayIcon object with the given \a icon and \a parent.

    The icon is initially invisible.

    \sa visible
*/
QSystemTrayIcon::QSystemTrayIcon(const QIcon &icon, QObject *parent)
    : QSystemTrayIcon(parent)
{
    setIcon(icon);
}

/*!
    Removes the icon from the system tray and frees all allocated resources.
*/
QSystemTrayIcon::~QSystemTrayIcon()
{
    Q_D(QSystemTrayIcon);
    d->remove_sys();
}

#if QT_CONFIG(menu)

/*!
    Sets the specified \a menu to be the context menu for the system tray icon.... }

/*!
    Returns the current context menu for the system tray entry.
*/
QMenu* QSystemTrayIcon::contextMenu() const
{
    Q_D(const QSystemTrayIcon);
    return d->menu;
}

#endif // QT_CONFIG(menu)

/*!
    \property QSystemTrayIcon::icon
    \brief the system tray icon

    On Windows, the system tray icon size is 16x16; on X11, the preferred size is
    22x22. The icon will be scaled to the appropriate size as necessary.
*/
void QSystemTrayIcon::setIcon(const QIcon &icon)
{
    Q_D(QSystemTrayIcon);
    d->icon = icon;
    d->updateIcon_sys();
}

QIcon QSystemTrayIcon::icon() const
{
    Q_D(const QSystemTrayIcon);
    return d->icon;
}

/*!
    \property QSystemTrayIcon::toolTip
    \brief the tooltip for the system tray entry

    On some systems, the tooltip's length is limited. The tooltip will be truncated
    if necessary.
*/
void QSystemTrayIcon::setToolTip(const QString &tooltip)
{
    Q_D(QSystemTrayIcon);
    d->toolTip = tooltip;
    d->updateToolTip_sys();
}... QString QSystemTrayIcon::toolTip() const
{
    Q_D(const QSystemTrayIcon);
    return d->toolTip;
}

/*!
    \fn void QSystemTrayIcon::show()

    Shows the icon in the system tray.

    \sa hide(), visible
*/

/*!
    \fn void QSystemTrayIcon::hide()

    Hides the system tray entry.

    \sa show(), visible
*/

/*!
    \since 4.3
    Returns the geometry of the system tray icon in screen coordinates.

    \sa visible
*/
QRect QSystemTrayIcon::geometry() const
{
    Q_D(const QSystemTrayIcon);
    if (!d->visible)
        return QRect();
    return d->geometry_sys();
}

/*!
    \property QSystemTrayIcon::visible
    \brief whether the system tray entry is visible

    Setting this property to true or calling show() makes the system tray icon
    visible; setting this property to false or calling hide() hides it.
*/
void QSystemTrayIcon::setVisible(bool visible)
{
    Q_D(QSystemTrayIcon);
    if (visible == d->visible)
        return;
    if (Q_UNLIKELY(visible && d->icon.isNull()))
        qWarning("QSystemTrayIcon::setVisible: No Icon set");
    d->visible = visible;
    if (d->visible)
        d->install_sys();
    else
        d->remove_sys();
}...   */
void QSystemTrayIcon::showMessage(const QString& title, const QString& msg,
                            QSystemTrayIcon::MessageIcon msgIcon, int msecs)
{
    Q_D(QSystemTrayIcon);
    if (d->visible)
        d->showMessage_sys(title, msg, messageIcon2qIcon(msgIcon), msgIcon, msecs);
}

/*!
    \fn void QSystemTrayIcon::showMessage(const QString &title, const QString &message, const QIcon &icon, int millisecondsTimeoutHint)

    \overload showMessage()

    Shows a balloon message for the entry with the given \a title, \a message,
    and custom icon \a icon for the time specified in \a millisecondsTimeoutHint.

    \since 5.9
*/
void QSystemTrayIcon::showMessage(const QString &title, const QString &msg,
                            const QIcon &icon, int msecs)
{
    Q_D(QSystemTrayIcon);
    if (d->visible)
        d->showMessage_sys(title, msg, icon, QSystemTrayIcon::NoIcon, msecs);
}

void QSystemTrayIconPrivate::_q_emitActivated(QPlatformSystemTrayIcon::ActivationReason reason)
{
    Q_Q(QSystemTrayIcon);
    emit q->activated(static_cast<QSystemTrayIcon::ActivationReason>(reason));
}

### 3. qtbase/src/widgets/util/qsystemtrayicon.cpp at dev · qt/qtbase

**URL:** https://github.com/qt/qtbase/blob/dev/src/widgets/util/qsystemtrayicon.cpp

// Copyright (C) 2016 The Qt Company Ltd.
// SPDX-License-Identifier: LicenseRef-Qt-Commercial OR LGPL-3.0-only OR GPL-2.0-only OR GPL-3.0-only

#include "qsystemtrayicon.h"
#include "qsystemtrayicon_p.h"

#ifndef QT_NO_SYSTEMTRAYICON

#if QT_CONFIG(menu)
#include "qmenu.h"
#endif
#include "qlist.h"
#include "qevent.h"
#include "qpoint.h"
#if QT_CONFIG(label)
#include "qlabel.h"
#include "private/qlabel_p.h"
#endif
#if QT_CONFIG(pushbutton)
#include "qpushbutton.h"
#endif
#include "qpainterpath.h"
#include "qpainter.h"
#include "qstyle.h"
#include "qgridlayout.h"
#include "qapplication.h"
#include "qbitmap.h"

#include <private/qhighdpiscaling_p.h>
#include <qpa/qplatformscreen.h>

using namespace std::chrono_literals;

QT_BEGIN_NAMESPACE

static QIcon messageIcon2qIcon(QSystemTrayIcon::MessageIcon icon)
{
    QStyle::StandardPixmap stdIcon = QStyle::SP_CustomBase; // silence gcc 4.9.0 about uninited variable
    switch (icon) {
    case QSystemTrayIcon::Information:
        stdIcon = QStyle::SP_MessageBoxInformation;
        break;
    case QSystemTrayIcon::Warning:
        stdIcon = QStyle::SP_MessageBoxWarning;
        break;
    case QSystemTrayIcon::Critical:
        stdIcon = QStyle::SP_MessageBoxCritical;
        break;
    case QSystemTrayIcon::NoIcon:
        return QIcon();
    }
    return QApplication::style()->standardIcon(stdIcon);
}... /*!
    \class QSystemTrayIcon
    \brief The QSystemTrayIcon class provides an icon for an application in the system tray.
    \since 4.2
    \ingroup desktop
    \inmodule QtWidgets...     Modern operating systems usually provide a special area on the desktop,
    called the \e{system tray} or \e{notification area}, where long-running
    applications can display icons and short messages.

    \image system-tray.webp The system tray on Windows 10.

    The QSystemTrayIcon class can be used on the following platforms:

    \list
    \li All supported versions of Windows.
    \li All Linux desktop environments that implement the D-Bus
       \l{http://www.freedesktop.org/wiki/Specifications/StatusNotifierItem/StatusNotifierItem}
       {StatusNotifierItem specification}, including KDE, Gnome, Xfce, LXQt, and DDE.
    \li All window managers and independent tray implementations for X11 that implement the
       \l{http://standards.freedesktop.org/systemtray-spec/systemtray-spec-0.2.html}
       {freedesktop.org XEmbed system tray specification}.
    \li All supported versions of \macos.
    \endlist

    To check whether a system tray is present on the user's desktop,
    call the QSystemTrayIcon::isSystemTrayAvailable() static function.

    To add a system tray entry, create a QSystemTrayIcon object, call setContextMenu()
    to provide a context menu for the icon, and call show() to make it visible in the
    system tray. Status notification messages ("balloon messages") can be displayed at
    any time using showMessage().

    If the system tray is unavailable when a system tray icon is constructed, but
    becomes available later, QSystemTrayIcon will automatically add an entry for the
    application in the system tray if the icon is \l visible.

    The activated() signal is emitted when the user activates the icon.

    Only on X11, when a tooltip is requested, the QSystemTrayIcon receives a QHelpEvent
    of type QEvent::ToolTip. Additionally, the QSystemTrayIcon receives wheel events of
    type QEvent::Wheel. These are not supported on any other platform. Note: Since GNOME
    Shell version 3.26, not all QSystemTrayIcon::ActivationReason are supported by the
    system without shell extensions installed.

    \sa QDesktopServices, {Desktop Integration}, {System Tray Icon Example}... */

/*!
    \enum QSystemTrayIcon::MessageIcon

    This enum describes the icon that is shown when a balloon message is displayed.

    \value NoIcon No icon is shown.
    \value Information An information icon is shown.
    \value Warning A standard warning icon is shown.
    \value Critical A critical warning icon is shown.

    \sa QMessageBox
*/

/*!
    Constructs a QSystemTrayIcon object with the given \a parent.

    The icon is initially invisible.

    \sa visible
*/
QSystemTrayIcon::QSystemTrayIcon(QObject *parent)
: QObject(*new QSystemTrayIconPrivate(), parent)
{
}

/*!
    Constructs a QSystemTrayIcon object with the given \a icon and \a parent.

    The icon is initially invisible.

    \sa visible
*/
QSystemTrayIcon::QSystemTrayIcon(const QIcon &icon, QObject *parent)
    : QSystemTrayIcon(parent)
{
    setIcon(icon);
}

/*!
    Removes the icon from the system tray and frees all allocated resources.
*/
QSystemTrayIcon::~QSystemTrayIcon()
{
    Q_D(QSystemTrayIcon);
    d->remove_sys();
}

#if QT_CONFIG(menu)

/*!
    Sets the specified \a menu to be the context menu for the system tray icon.... }

/*!
    Returns the current context menu for the system tray entry.
*/
QMenu* QSystemTrayIcon::contextMenu() const
{
    Q_D(const QSystemTrayIcon);
    return d->menu;
}

#endif // QT_CONFIG(menu)

/*!
    \property QSystemTrayIcon::icon
    \brief the system tray icon

    On Windows, the system tray icon size is 16x16; on X11, the preferred size is
    22x22. The icon will be scaled to the appropriate size as necessary.
*/
void QSystemTrayIcon::setIcon(const QIcon &icon)
{
    Q_D(QSystemTrayIcon);
    d->icon = icon;
    d->updateIcon_sys();
}

QIcon QSystemTrayIcon::icon() const
{
    Q_D(const QSystemTrayIcon);
    return d->icon;
}

/*!
    \property QSystemTrayIcon::toolTip
    \brief the tooltip for the system tray entry

    On some systems, the tooltip's length is limited. The tooltip will be truncated
    if necessary.
*/
void QSystemTrayIcon::setToolTip(const QString &tooltip)
{
    Q_D(QSystemTrayIcon);
    d->toolTip = tooltip;
    d->updateToolTip_sys();
}... QString QSystemTrayIcon::toolTip() const
{
    Q_D(const QSystemTrayIcon);
    return d->toolTip;
}

/*!
    \fn void QSystemTrayIcon::show()

    Shows the icon in the system tray.

    \sa hide(), visible
*/

/*!
    \fn void QSystemTrayIcon::hide()

    Hides the system tray entry.

    \sa show(), visible
*/

/*!
    \since 4.3
    Returns the geometry of the system tray icon in screen coordinates.

    \sa visible
*/
QRect QSystemTrayIcon::geometry() const
{
    Q_D(const QSystemTrayIcon);
    if (!d->visible)
        return QRect();
    return d->geometry_sys();
}

/*!
    \property QSystemTrayIcon::visible
    \brief whether the system tray entry is visible

    Setting this property to true or calling show() makes the system tray icon
    visible; setting this property to false or calling hide() hides it.
*/
void QSystemTrayIcon::setVisible(bool visible)
{
    Q_D(QSystemTrayIcon);
    if (visible == d->visible)
        return;
    if (Q_UNLIKELY(visible && d->icon.isNull()))
        qWarning("QSystemTrayIcon::setVisible: No Icon set");
    d->visible = visible;
    if (d->visible)
        d->install_sys();
    else
        d->remove_sys();
}...   */
void QSystemTrayIcon::showMessage(const QString& title, const QString& msg,
                            QSystemTrayIcon::MessageIcon msgIcon, int msecs)
{
    Q_D(QSystemTrayIcon);
    if (d->visible)
        d->showMessage_sys(title, msg, messageIcon2qIcon(msgIcon), msgIcon, msecs);
}

/*!
    \fn void QSystemTrayIcon::showMessage(const QString &title, const QString &message, const QIcon &icon, int millisecondsTimeoutHint)

    \overload showMessage()

    Shows a balloon message for the entry with the given \a title, \a message,
    and custom icon \a icon for the time specified in \a millisecondsTimeoutHint.

    \since 5.9
*/
void QSystemTrayIcon::showMessage(const QString &title, const QString &msg,
                            const QIcon &icon, int msecs)
{
    Q_D(QSystemTrayIcon);
    if (d->visible)
        d->showMessage_sys(title, msg, icon, QSystemTrayIcon::NoIcon, msecs);
}

void QSystemTrayIconPrivate::_q_emitActivated(QPlatformSystemTrayIcon::ActivationReason reason)
{
    Q_Q(QSystemTrayIcon);
    emit q->activated(static_cast<QSystemTrayIcon::ActivationReason>(reason));
}

### 4. QSystemTrayIcon, change behavior - Qt Forum

**URL:** https://forum.qt.io/topic/123729/qsystemtrayicon-change-behavior

# QSystemTrayIcon, change behavior



I just learned how use the QSystemTrayIcon, however it works in a way that is not exactly what I want.

This is the code(snippet) that I am using :

`void MainWindow::on_button_clicked() { // tray is declared in the class and initialized with the icon in the constructor tray->show(); tray->showMessage("A test", "this has proved to be working", QSystemTrayIcon::NoIcon ); / tray->setVisible(false); }`

With this code the tray is shown as expected but after the tray disappears the icon stay here:

which is something I don't want since it has no use there(didn't set any contextmenu)

If after I

`showMessage()`I set the tray to invisible it removes the icon from there but the notification won't stay in the notification area.

Is there a way to change this behavior?



Hi,

A tray icon does not necessarily have a contextual menu, it might also just provide some information.

Your situation is not really clear.

What do you want from your system tray icon ?



Hi,

A tray icon does not necessarily have a contextual menu, it might also just provide some information.

Your situation is not really clear.

What do you want from your system tray icon ?... @SGaist thanks for the quick reply

What do you want from your system tray icon ?

This is the message it shows.

After this message disappears, the icon I set for the tray continues here:

I don't want that.

I can set the visibility to false after

`showMessage()`but then, the message won't stay in the notification area.

I hope I made the question clear.



So you just want the notification but not the icon ?



Then you might have to pilfer the backend code of QSystemTrayIcon since you have no use of it beside the notification part.



Then you might have to pilfer the backend code of QSystemTrayIcon since you have no use of it beside the notification part.



Then you might have to pilfer the backend code of QSystemTrayIcon since you have no use of it beside the notification part.

Where do I find the code?Lifetime Qt Champion

In the git repo or e.g. here: https://code.woboq.org/qt5/qtbase/src/widgets/util/qsystemtrayicon.cpp.html



In the git repo or e.g. here: https://code.woboq.org/qt5/qtbase/src/widgets/util/qsystemtrayicon.cpp.html

@Christian-Ehrlicher thank you... Is there any step by step on how to build it from source, or in which category here in the forum can I post a question?



Lifetime Qt Champion



@Christian-Ehrlicher just out of curiosity



Then you might have to pilfer the backend code of QSystemTrayIcon since you have no use of it beside the notification part.



@hbatalha said in QSystemTrayIcon, change behavior:

What exactly did you mean by "pilfer the backend "? Should I remove some code?

No, you should get some inspiration from it.



@hbatalha said in QSystemTrayIcon, change behavior:

What exactly did you mean by "pilfer the backend "? Should I remove some code?

No, you should get some inspiration from it.



You have to do some more spelunking and go to the backend side of things to get the Windows specific implementation.



You have to do some more spelunking and go to the backend side of things to get the Windows specific implementation.

### 5. QSystemTrayIcon, change behavior - Qt Forum

**URL:** https://forum.qt.io/topic/123729/qsystemtrayicon-change-behavior

# QSystemTrayIcon, change behavior



I just learned how use the QSystemTrayIcon, however it works in a way that is not exactly what I want.

This is the code(snippet) that I am using :

`void MainWindow::on_button_clicked() { // tray is declared in the class and initialized with the icon in the constructor tray->show(); tray->showMessage("A test", "this has proved to be working", QSystemTrayIcon::NoIcon ); / tray->setVisible(false); }`

With this code the tray is shown as expected but after the tray disappears the icon stay here:

which is something I don't want since it has no use there(didn't set any contextmenu)

If after I

`showMessage()`I set the tray to invisible it removes the icon from there but the notification won't stay in the notification area.

Is there a way to change this behavior?



Hi,

A tray icon does not necessarily have a contextual menu, it might also just provide some information.

Your situation is not really clear.

What do you want from your system tray icon ?



Hi,

A tray icon does not necessarily have a contextual menu, it might also just provide some information.

Your situation is not really clear.

What do you want from your system tray icon ?... @SGaist thanks for the quick reply

What do you want from your system tray icon ?

This is the message it shows.

After this message disappears, the icon I set for the tray continues here:

I don't want that.

I can set the visibility to false after

`showMessage()`but then, the message won't stay in the notification area.

I hope I made the question clear.



So you just want the notification but not the icon ?



Then you might have to pilfer the backend code of QSystemTrayIcon since you have no use of it beside the notification part.



Then you might have to pilfer the backend code of QSystemTrayIcon since you have no use of it beside the notification part.



Then you might have to pilfer the backend code of QSystemTrayIcon since you have no use of it beside the notification part.

Where do I find the code?Lifetime Qt Champion

In the git repo or e.g. here: https://code.woboq.org/qt5/qtbase/src/widgets/util/qsystemtrayicon.cpp.html



In the git repo or e.g. here: https://code.woboq.org/qt5/qtbase/src/widgets/util/qsystemtrayicon.cpp.html

@Christian-Ehrlicher thank you... Is there any step by step on how to build it from source, or in which category here in the forum can I post a question?



Lifetime Qt Champion



@Christian-Ehrlicher just out of curiosity



Then you might have to pilfer the backend code of QSystemTrayIcon since you have no use of it beside the notification part.



@hbatalha said in QSystemTrayIcon, change behavior:

What exactly did you mean by "pilfer the backend "? Should I remove some code?

No, you should get some inspiration from it.



@hbatalha said in QSystemTrayIcon, change behavior:

What exactly did you mean by "pilfer the backend "? Should I remove some code?

No, you should get some inspiration from it.



You have to do some more spelunking and go to the backend side of things to get the Windows specific implementation.



You have to do some more spelunking and go to the backend side of things to get the Windows specific implementation.

## Metadata

```json
{
  "planId": "plan_2",
  "executionTime": 55187,
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
