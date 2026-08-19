#pragma once

#include <QObject>
#include <QAction>
#include <QList>
#include <QSystemTrayIcon>
#include <QMenu>
#include "HeadsetClient.h"

class EqDialog;

// System tray application. Uses a plain headset icon and opens its menu on click.
// Provides battery percentage, quick toggles, and EQ/sidetone submenus.
class TrayApp : public QObject {
    Q_OBJECT
public:
    explicit TrayApp(HeadsetClient* client, QObject* parent = nullptr);

private slots:
    void onBatteryChanged(int percent);
    void onConnectedChanged(bool connected);
    void openEqDialog();

private:
    void buildMenu();
    void updateTooltip();
    void syncSettings();
    void populateSavedProfiles();

    HeadsetClient* m_client;
    QSystemTrayIcon* m_tray;
    QMenu* m_menu;
    QAction* m_batteryAction; // battery display in the menu
    QMenu* m_savedProfilesMenu = nullptr;
    QList<QAction*> m_eqPresetActions;
    QList<QAction*> m_sidetoneActions;
    QList<QAction*> m_sleepActions;
    QAction* m_noiseCancelAction = nullptr;
    QAction* m_powerSaveAction = nullptr;
    EqDialog* m_eqDialog = nullptr;
};
