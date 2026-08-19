#include "TrayApp.h"
#include "EqDialog.h"

#include <QApplication>
#include <QIcon>
#include <QActionGroup>
#include <QSignalBlocker>

TrayApp::TrayApp(HeadsetClient* client, QObject* parent)
    : QObject(parent), m_client(client) {
    m_tray = new QSystemTrayIcon(this);

    // Plain headset icon. Prefer the theme icon, then fall back to the embedded SVG.
    QIcon icon = QIcon::fromTheme("audio-headset");
    if (icon.isNull()) {
        icon = QIcon(":/assets/headset.svg");
    }
    m_tray->setIcon(icon);

    buildMenu();
    m_tray->setContextMenu(m_menu);
    updateTooltip();
    m_tray->show();

    connect(m_client, &HeadsetClient::batteryChanged, this,
            &TrayApp::onBatteryChanged);
    connect(m_client, &HeadsetClient::connectedChanged, this,
            &TrayApp::onConnectedChanged);
    connect(m_client, &HeadsetClient::settingsChanged, this, &TrayApp::syncSettings);
    syncSettings();
}

void TrayApp::buildMenu() {
    m_menu = new QMenu();

    // Battery display (non-clickable heading).
    m_batteryAction = m_menu->addAction("Battery: ...");
    m_batteryAction->setEnabled(false);
    m_menu->addSeparator();

    // --- EQ preset submenu ---
    QMenu* eqMenu = m_menu->addMenu("EQ Preset");
    auto* eqGroup = new QActionGroup(this);
    eqGroup->setExclusive(true);
    const QStringList presets = {"default", "game", "movie", "music"};
    for (const QString& p : presets) {
        QAction* a = eqMenu->addAction(p);
        a->setCheckable(true);
        a->setData(p);
        eqGroup->addAction(a);
        m_eqPresetActions.append(a);
        connect(a, &QAction::triggered, this, [this, p]() {
            m_client->setEqPreset(p);
        });
    }
    eqMenu->addSeparator();
    m_savedProfilesMenu = eqMenu->addMenu("Saved profiles");
    connect(m_savedProfilesMenu, &QMenu::aboutToShow, this, &TrayApp::populateSavedProfiles);
    QAction* customEq = eqMenu->addAction("Custom EQ…");
    connect(customEq, &QAction::triggered, this, &TrayApp::openEqDialog);

    // StatusNotifier menus do not support embedded QWidget controls reliably.
    // Use direct, checkable levels instead so every compositor renders them.
    QMenu* sideMenu = m_menu->addMenu("Sidetone");
    auto* sidetoneGroup = new QActionGroup(this);
    sidetoneGroup->setExclusive(true);
    for (int level = 0; level <= 15; ++level) {
        QAction* action = sideMenu->addAction(level == 0 ? "Off" : QString::number(level));
        action->setCheckable(true);
        action->setData(level);
        sidetoneGroup->addAction(action);
        m_sidetoneActions.append(action);
        connect(action, &QAction::triggered, this,
                [this, level]() { m_client->setSidetone(level); });
    }

    // --- Noise cancellation toggle ---
    m_noiseCancelAction = m_menu->addAction("Noise cancellation");
    m_noiseCancelAction->setCheckable(true);
    connect(m_noiseCancelAction, &QAction::toggled, this,
            [this](bool on) { m_client->setNoiseCancel(on); });

    // --- Power-saving controls ---
    QMenu* powerMenu = m_menu->addMenu("Power");
    m_powerSaveAction = powerMenu->addAction("Battery saver");
    m_powerSaveAction->setCheckable(true);
    connect(m_powerSaveAction, &QAction::toggled, this,
            [this](bool on) { m_client->setPowerSave(on); });

    powerMenu->addSeparator();
    QAction* timeoutLabel = powerMenu->addAction("Turn off after:");
    timeoutLabel->setEnabled(false);
    auto* sleepGroup = new QActionGroup(this);
    sleepGroup->setExclusive(true);
    const QList<QPair<QString, int>> sleeps = {
        {"Never", 0}, {"15 minutes", 15}, {"30 minutes", 30},
        {"45 minutes", 45}, {"60 minutes", 60}};
    for (const auto& s : sleeps) {
        QAction* a = powerMenu->addAction(s.first);
        a->setCheckable(true);
        a->setData(s.second);
        sleepGroup->addAction(a);
        m_sleepActions.append(a);
        int mins = s.second;
        connect(a, &QAction::triggered, this,
                [this, mins]() { m_client->setSleep(mins); });
    }

    m_menu->addSeparator();
    QAction* refresh = m_menu->addAction("Refresh battery");
    connect(refresh, &QAction::triggered, this,
            [this]() { m_client->refreshBattery(); });

    m_menu->addSeparator();
    QAction* quit = m_menu->addAction("Quit");
    connect(quit, &QAction::triggered, qApp, &QApplication::quit);
}

void TrayApp::openEqDialog() {
    if (!m_eqDialog) {
        m_eqDialog = new EqDialog(m_client);
    }
    m_eqDialog->show();
    m_eqDialog->raise();
    m_eqDialog->activateWindow();
}

void TrayApp::updateTooltip() {
    int b = m_client->battery();
    bool c = m_client->connected();
    QString batt = (b >= 0) ? QString("%1%").arg(b) : "?";
    m_tray->setToolTip(QString("%1  |  Battery: %2")
                           .arg(c ? "Connected" : "Disconnected", batt));
    m_batteryAction->setText(
        c ? QString("Battery: %1").arg(batt) : QString("Disconnected"));
}

void TrayApp::syncSettings() {
    for (QAction* action : m_eqPresetActions) {
        action->setChecked(action->data().toString() == m_client->eqPreset());
    }
    for (QAction* action : m_sidetoneActions) {
        action->setChecked(action->data().toInt() == m_client->sidetone());
    }
    for (QAction* action : m_sleepActions) {
        action->setChecked(action->data().toInt() == m_client->sleepMinutes());
    }
    const QSignalBlocker noiseCancelBlocker(m_noiseCancelAction);
    const QSignalBlocker powerSaveBlocker(m_powerSaveAction);
    m_noiseCancelAction->setChecked(m_client->noiseCancel());
    m_powerSaveAction->setChecked(m_client->powerSave());
}

void TrayApp::populateSavedProfiles() {
    m_savedProfilesMenu->clear();
    const QStringList profiles = m_client->eqProfiles();
    if (profiles.isEmpty()) {
        QAction* empty = m_savedProfilesMenu->addAction("No saved profiles");
        empty->setEnabled(false);
        return;
    }
    for (const QString& name : profiles) {
        QAction* action = m_savedProfilesMenu->addAction(name);
        connect(action, &QAction::triggered, this,
                [this, name]() { m_client->applyEqProfile(name); });
    }
}

void TrayApp::onBatteryChanged(int) { updateTooltip(); }
void TrayApp::onConnectedChanged(bool) { updateTooltip(); }
