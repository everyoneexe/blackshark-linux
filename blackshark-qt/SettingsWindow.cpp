#include "SettingsWindow.h"
#include "EqGraph.h"

#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QFormLayout>
#include <QGroupBox>
#include <QLabel>
#include <QSlider>
#include <QCheckBox>
#include <QComboBox>
#include <QPushButton>
#include <array>

SettingsWindow::SettingsWindow(HeadsetClient* client, QWidget* parent)
    : QWidget(parent), m_client(client) {
    setWindowTitle("BlackShark V3 X - Settings");
    setMinimumWidth(720);

    auto* root = new QVBoxLayout(this);

    // --- Status ---
    m_status = new QLabel(this);
    m_status->setStyleSheet("font-weight: bold;");
    root->addWidget(m_status);
    updateStatusLabel();

    // --- Sound / EQ ---
    auto* soundBox = new QGroupBox("Audio Equalizer", this);
    auto* soundLayout = new QVBoxLayout(soundBox);

    // Preset buttons row + Reset
    auto* presetRow = new QHBoxLayout();
    const QStringList presets = {"default", "game", "movie", "music"};
    for (const QString& name : presets) {
        auto* btn = new QPushButton(name.toUpper(), soundBox);
        btn->setCheckable(true);
        btn->setProperty("presetName", name);
        btn->setStyleSheet(
            "QPushButton { padding: 6px 12px; border: 1px solid #3a3c42;"
            " border-radius: 6px; }"
            "QPushButton:checked { border: 2px solid #43b581; color: #43b581; }");
        m_presetButtons.append(btn);
        presetRow->addWidget(btn);
        connect(btn, &QPushButton::clicked, this, [this, name]() {
            m_client->setEqPreset(name);
            selectPreset(name);
            // Presets flatten the custom curve in the daemon config; show flat.
            m_eq->setBands(std::array<int, EqGraph::BANDS>{});
        });
    }
    presetRow->addStretch();
    auto* resetBtn = new QPushButton("RESET", soundBox);
    presetRow->addWidget(resetBtn);
    connect(resetBtn, &QPushButton::clicked, this, [this]() {
        m_eq->resetFlat(); // sets flat and emits eqCommitted
    });

    soundLayout->addLayout(presetRow);

    // Graphical EQ
    m_eq = new EqGraph(soundBox);
    soundLayout->addWidget(m_eq);
    connect(m_eq, &EqGraph::eqCommitted, this, [this](QList<int> bands) {
        m_client->setEq(bands);
        // Dragging the curve means a custom EQ; no preset is active anymore.
        selectPreset(QString());
    });

    root->addWidget(soundBox);

    // --- Microphone ---
    auto* micBox = new QGroupBox("Microphone", this);
    auto* micForm = new QFormLayout(micBox);

    auto* sideRow = new QHBoxLayout();
    m_sidetone = new QSlider(Qt::Horizontal, micBox);
    m_sidetone->setRange(0, 15);
    m_sidetoneVal = new QLabel("0", micBox);
    m_sidetoneVal->setMinimumWidth(24);
    sideRow->addWidget(m_sidetone);
    sideRow->addWidget(m_sidetoneVal);
    micForm->addRow("Sidetone:", sideRow);
    connect(m_sidetone, &QSlider::valueChanged, this, [this](int v) {
        m_sidetoneVal->setText(QString::number(v));
    });
    connect(m_sidetone, &QSlider::sliderReleased, this, [this]() {
        m_client->setSidetone(m_sidetone->value());
    });

    m_noiseCancel = new QCheckBox("Noise cancellation", micBox);
    micForm->addRow(m_noiseCancel);
    connect(m_noiseCancel, &QCheckBox::toggled, this,
            [this](bool on) { m_client->setNoiseCancel(on); });

    root->addWidget(micBox);

    // --- Power ---
    auto* powerBox = new QGroupBox("Power", this);
    auto* powerForm = new QFormLayout(powerBox);

    m_sleep = new QComboBox(powerBox);
    m_sleep->addItem("Off", 0);
    m_sleep->addItem("15 min", 15);
    m_sleep->addItem("30 min", 30);
    m_sleep->addItem("45 min", 45);
    m_sleep->addItem("60 min", 60);
    powerForm->addRow("Sleep:", m_sleep);
    connect(m_sleep, QOverload<int>::of(&QComboBox::currentIndexChanged), this,
            [this](int) { m_client->setSleep(m_sleep->currentData().toInt()); });

    m_powerSave = new QCheckBox("Power saving", powerBox);
    powerForm->addRow(m_powerSave);
    connect(m_powerSave, &QCheckBox::toggled, this,
            [this](bool on) { m_client->setPowerSave(on); });

    root->addWidget(powerBox);
    root->addStretch();

    // Default selection
    selectPreset("default");

    // Listen to daemon signals.
    connect(m_client, &HeadsetClient::batteryChanged, this,
            &SettingsWindow::onBatteryChanged);
    connect(m_client, &HeadsetClient::connectedChanged, this,
            &SettingsWindow::onConnectedChanged);
}

void SettingsWindow::selectPreset(const QString& name) {
    m_activePreset = name;
    for (QPushButton* btn : m_presetButtons) {
        bool active = (btn->property("presetName").toString() == name);
        btn->setChecked(active);
    }
}

void SettingsWindow::updateStatusLabel() {
    int b = m_client->battery();
    bool c = m_client->connected();
    QString batt = (b >= 0) ? QString("%1%").arg(b) : "unknown";
    m_status->setText(QString("Status: %1  |  Battery: %2")
                          .arg(c ? "Connected" : "Disconnected", batt));
}

void SettingsWindow::onBatteryChanged(int) { updateStatusLabel(); }
void SettingsWindow::onConnectedChanged(bool) { updateStatusLabel(); }
