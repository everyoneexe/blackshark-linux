#include "TrayApp.h"
#include "SettingsWindow.h"

#include <QApplication>
#include <QIcon>
#include <QActionGroup>
#include <QWidgetAction>
#include <QSlider>
#include <QLabel>
#include <QVBoxLayout>
#include <QWidget>

TrayApp::TrayApp(HeadsetClient* client, QObject* parent)
    : QObject(parent), m_client(client) {
    m_tray = new QSystemTrayIcon(this);

    // Duz kulaklik ikonu. Once tema ikonu, yoksa gomulu SVG kaynagi.
    QIcon icon = QIcon::fromTheme("audio-headset");
    if (icon.isNull()) {
        icon = QIcon(":/assets/headset.svg");
    }
    m_tray->setIcon(icon);

    buildMenu();
    m_tray->setContextMenu(m_menu);
    updateTooltip();
    m_tray->show();

    // Sol tik ile de ayarlar acilsin.
    connect(m_tray, &QSystemTrayIcon::activated, this,
            [this](QSystemTrayIcon::ActivationReason reason) {
                if (reason == QSystemTrayIcon::Trigger) openSettings();
            });

    connect(m_client, &HeadsetClient::batteryChanged, this,
            &TrayApp::onBatteryChanged);
    connect(m_client, &HeadsetClient::connectedChanged, this,
            &TrayApp::onConnectedChanged);
}

void TrayApp::buildMenu() {
    m_menu = new QMenu();

    // Batarya gosterimi (tiklanamaz baslik)
    m_batteryAction = m_menu->addAction("Batarya: ...");
    m_batteryAction->setEnabled(false);
    m_menu->addSeparator();

    // --- EQ preset alt menusu ---
    QMenu* eqMenu = m_menu->addMenu("EQ Preset");
    auto* eqGroup = new QActionGroup(this);
    eqGroup->setExclusive(true);
    const QStringList presets = {"default", "game", "movie", "music"};
    for (const QString& p : presets) {
        QAction* a = eqMenu->addAction(p);
        a->setCheckable(true);
        eqGroup->addAction(a);
        connect(a, &QAction::triggered, this, [this, p]() {
            m_client->setEqPreset(p);
        });
    }

    // --- Sidetone alt menusu (slider gomulu) ---
    QMenu* sideMenu = m_menu->addMenu("Sidetone");
    {
        auto* w = new QWidget();
        auto* lay = new QVBoxLayout(w);
        lay->setContentsMargins(8, 4, 8, 4);
        auto* lbl = new QLabel("Seviye: 0", w);
        auto* slider = new QSlider(Qt::Horizontal, w);
        slider->setRange(0, 15);
        lay->addWidget(lbl);
        lay->addWidget(slider);
        connect(slider, &QSlider::valueChanged, w,
                [lbl](int v) { lbl->setText(QString("Seviye: %1").arg(v)); });
        connect(slider, &QSlider::sliderReleased, this,
                [this, slider]() { m_client->setSidetone(slider->value()); });
        auto* wa = new QWidgetAction(this);
        wa->setDefaultWidget(w);
        sideMenu->addAction(wa);
    }

    // --- Noise cancellation toggle ---
    QAction* nc = m_menu->addAction("Gurultu engelleme");
    nc->setCheckable(true);
    connect(nc, &QAction::toggled, this,
            [this](bool on) { m_client->setNoiseCancel(on); });

    // --- Power save toggle ---
    QAction* ps = m_menu->addAction("Guc tasarrufu");
    ps->setCheckable(true);
    connect(ps, &QAction::toggled, this,
            [this](bool on) { m_client->setPowerSave(on); });

    // --- Uyku alt menusu ---
    QMenu* sleepMenu = m_menu->addMenu("Uyku");
    auto* sleepGroup = new QActionGroup(this);
    sleepGroup->setExclusive(true);
    const QList<QPair<QString, int>> sleeps = {
        {"Kapali", 0}, {"15 dk", 15}, {"30 dk", 30}, {"45 dk", 45}, {"60 dk", 60}};
    for (const auto& s : sleeps) {
        QAction* a = sleepMenu->addAction(s.first);
        a->setCheckable(true);
        sleepGroup->addAction(a);
        int mins = s.second;
        connect(a, &QAction::triggered, this,
                [this, mins]() { m_client->setSleep(mins); });
    }

    m_menu->addSeparator();
    QAction* settings = m_menu->addAction("Ayarlar...");
    connect(settings, &QAction::triggered, this, &TrayApp::openSettings);

    QAction* refresh = m_menu->addAction("Bataryayi yenile");
    connect(refresh, &QAction::triggered, this,
            [this]() { m_client->refreshBattery(); });

    m_menu->addSeparator();
    QAction* quit = m_menu->addAction("Cikis");
    connect(quit, &QAction::triggered, qApp, &QApplication::quit);
}

void TrayApp::openSettings() {
    if (!m_settings) {
        m_settings = new SettingsWindow(m_client);
    }
    m_settings->show();
    m_settings->raise();
    m_settings->activateWindow();
}

void TrayApp::updateTooltip() {
    int b = m_client->battery();
    bool c = m_client->connected();
    QString batt = (b >= 0) ? QString("%%1").arg(b) : "?";
    m_tray->setToolTip(QString("BlackShark V3 X\n%1  |  Batarya: %2")
                           .arg(c ? "Bagli" : "Bagli degil", batt));
    m_batteryAction->setText(
        c ? QString("Batarya: %1").arg(batt) : QString("Bagli degil"));
}

void TrayApp::onBatteryChanged(int) { updateTooltip(); }
void TrayApp::onConnectedChanged(bool) { updateTooltip(); }
