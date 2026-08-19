#pragma once

#include <QObject>
#include <QSystemTrayIcon>
#include <QMenu>
#include "HeadsetClient.h"

class SettingsWindow;

// System tray application. Uses a plain headset icon and opens its menu on click.
// Provides battery percentage, quick toggles, and EQ/sidetone submenus.
class TrayApp : public QObject {
    Q_OBJECT
public:
    explicit TrayApp(HeadsetClient* client, QObject* parent = nullptr);

private slots:
    void onBatteryChanged(int percent);
    void onConnectedChanged(bool connected);
    void openSettings();

private:
    void buildMenu();
    void updateTooltip();

    HeadsetClient* m_client;
    QSystemTrayIcon* m_tray;
    QMenu* m_menu;
    QAction* m_batteryAction; // battery display in the menu
    SettingsWindow* m_settings = nullptr;
};
