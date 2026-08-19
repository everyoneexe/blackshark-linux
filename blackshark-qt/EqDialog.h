#pragma once

#include <QDialog>
#include <QList>

class HeadsetClient;
class QComboBox;
class QSpinBox;

// A small, precise editor opened only from the tray's Equalizer menu.
class EqDialog : public QDialog {
    Q_OBJECT
public:
    explicit EqDialog(HeadsetClient* client, QWidget* parent = nullptr);

private:
    void syncFromConfig();
    void reloadProfiles();
    void loadProfile(const QString& name);
    QList<int> bands() const;

    HeadsetClient* m_client;
    QComboBox* m_profiles;
    QList<QSpinBox*> m_bands;
};
