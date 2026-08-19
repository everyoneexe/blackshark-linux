#pragma once

#include <QWidget>
#include "HeadsetClient.h"

class QLabel;
class QSlider;
class QCheckBox;
class QComboBox;
class QPushButton;
class EqGraph;

// Settings window, opened from the tray "Settings..." entry.
class SettingsWindow : public QWidget {
    Q_OBJECT
public:
    explicit SettingsWindow(HeadsetClient* client, QWidget* parent = nullptr);

private slots:
    void onBatteryChanged(int percent);
    void onConnectedChanged(bool connected);

private:
    void updateStatusLabel();
    void selectPreset(const QString& name); // highlight active preset button

    HeadsetClient* m_client;

    QLabel* m_status;

    // EQ
    QList<QPushButton*> m_presetButtons;
    QString m_activePreset;
    EqGraph* m_eq;

    // Mic
    QSlider* m_sidetone;
    QLabel* m_sidetoneVal;
    QCheckBox* m_noiseCancel;

    // Power
    QComboBox* m_sleep;
    QCheckBox* m_powerSave;
};
