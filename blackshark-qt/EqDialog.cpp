#include "EqDialog.h"
#include "HeadsetClient.h"

#include <QDialogButtonBox>
#include <QComboBox>
#include <QGridLayout>
#include <QLabel>
#include <QPushButton>
#include <QSignalBlocker>
#include <QSpinBox>
#include <QVBoxLayout>

namespace {
const char* kFrequencies[] = {
    "31 Hz", "63 Hz", "125 Hz", "250 Hz", "500 Hz",
    "1 kHz", "2 kHz", "4 kHz", "8 kHz", "16 kHz",
};
constexpr int kBandCount = 10;
}

EqDialog::EqDialog(HeadsetClient* client, QWidget* parent)
    : QDialog(parent), m_client(client) {
    setWindowTitle("Custom EQ");
    setModal(false);
    setMinimumWidth(430);
    setStyleSheet(
        "QDialog { background: #111415; color: #e6e9e6; }"
        "QLabel#hint { color: #9ba6a1; }"
        "QSpinBox { background: #202625; border: 1px solid #394440; border-radius: 6px;"
        " padding: 5px; min-width: 65px; }"
        "QPushButton { background: #202625; border: 1px solid #394440; border-radius: 6px; padding: 7px 12px; }"
        "QPushButton#apply { background: #1d3b2e; border-color: #5ccf9a; color: #78e7b4; font-weight: 600; }"
        "QPushButton:hover { border-color: #5ccf9a; }");

    auto* root = new QVBoxLayout(this);
    root->setContentsMargins(18, 18, 18, 16);
    root->setSpacing(12);
    auto* hint = new QLabel("Set each band from −6 to +6 dB.", this);
    hint->setObjectName("hint");
    root->addWidget(hint);

    auto* profileRow = new QHBoxLayout();
    profileRow->addWidget(new QLabel("Profile:", this));
    m_profiles = new QComboBox(this);
    m_profiles->setEditable(true);
    m_profiles->setInsertPolicy(QComboBox::NoInsert);
    m_profiles->setPlaceholderText("Name a profile");
    profileRow->addWidget(m_profiles, 1);
    auto* saveProfile = new QPushButton("Save", this);
    auto* deleteProfile = new QPushButton("Delete", this);
    profileRow->addWidget(saveProfile);
    profileRow->addWidget(deleteProfile);
    root->addLayout(profileRow);

    auto* grid = new QGridLayout();
    grid->setHorizontalSpacing(12);
    grid->setVerticalSpacing(8);
    for (int i = 0; i < kBandCount; ++i) {
        const int row = i / 5;
        const int column = (i % 5) * 2;
        auto* label = new QLabel(kFrequencies[i], this);
        auto* spinBox = new QSpinBox(this);
        spinBox->setRange(-6, 6);
        spinBox->setSuffix(" dB");
        grid->addWidget(label, row, column);
        grid->addWidget(spinBox, row, column + 1);
        m_bands.append(spinBox);
    }
    root->addLayout(grid);

    auto* buttons = new QDialogButtonBox(this);
    auto* reset = buttons->addButton("Reset", QDialogButtonBox::ResetRole);
    auto* apply = buttons->addButton("Apply", QDialogButtonBox::ApplyRole);
    apply->setObjectName("apply");
    root->addWidget(buttons);

    connect(reset, &QPushButton::clicked, this, [this]() {
        for (QSpinBox* band : m_bands) band->setValue(0);
    });
    connect(apply, &QPushButton::clicked, this, [this]() {
        m_client->setEq(bands());
    });
    connect(saveProfile, &QPushButton::clicked, this, [this]() {
        const QString name = m_profiles->currentText().trimmed();
        if (name.isEmpty()) return;
        m_client->saveEqProfile(name, bands());
        reloadProfiles();
        m_profiles->setCurrentText(name);
    });
    connect(deleteProfile, &QPushButton::clicked, this, [this]() {
        const QString name = m_profiles->currentText().trimmed();
        if (name.isEmpty()) return;
        m_client->deleteEqProfile(name);
        reloadProfiles();
    });
    connect(m_profiles, QOverload<int>::of(&QComboBox::activated), this,
            [this](int index) {
                if (index >= 0) loadProfile(m_profiles->itemText(index));
            });
    connect(m_client, &HeadsetClient::settingsChanged, this, &EqDialog::syncFromConfig);
    reloadProfiles();
    syncFromConfig();
}

void EqDialog::syncFromConfig() {
    const QList<int> savedBands = m_client->eqBands();
    for (int i = 0; i < m_bands.size(); ++i) {
        const QSignalBlocker blocker(m_bands[i]);
        m_bands[i]->setValue(i < savedBands.size() ? savedBands[i] : 0);
    }
}

void EqDialog::reloadProfiles() {
    const QString current = m_profiles->currentText();
    const QSignalBlocker blocker(m_profiles);
    m_profiles->clear();
    m_profiles->addItems(m_client->eqProfiles());
    m_profiles->setCurrentText(current);
}

void EqDialog::loadProfile(const QString& name) {
    const QList<int> savedBands = m_client->eqProfile(name);
    if (savedBands.size() != kBandCount) return;
    for (int i = 0; i < m_bands.size(); ++i) {
        const QSignalBlocker blocker(m_bands[i]);
        m_bands[i]->setValue(savedBands[i]);
    }
}

QList<int> EqDialog::bands() const {
    QList<int> values;
    for (const QSpinBox* band : m_bands) values.append(band->value());
    return values;
}
