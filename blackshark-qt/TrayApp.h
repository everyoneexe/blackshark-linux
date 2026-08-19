#pragma once

#include <QObject>
#include <QSystemTrayIcon>
#include <QMenu>
#include "HeadsetClient.h"

class SettingsWindow;

// Sistem tepsisi. Duz kulaklik ikonu; tiklayinca menu acilir.
// Referans (RiskRunner0/blackshark-linux) tray'ine benzer: batarya %,
// hizli toggle'lar, EQ/sidetone alt menuleri.
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
    QAction* m_batteryAction; // menuda batarya gosterimi
    SettingsWindow* m_settings = nullptr;
};
