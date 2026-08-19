#pragma once

#include <QObject>
#include <QString>
#include <QtDBus/QDBusInterface>
#include <QtDBus/QDBusConnection>

// D-Bus client for the net.blackshark1 daemon.
// The tray and GUI share it. Neither touches HID directly; all requests use the daemon.
class HeadsetClient : public QObject {
    Q_OBJECT
public:
    static constexpr const char* SERVICE = "net.blackshark1";
    static constexpr const char* PATH = "/net/blackshark1/Headset";
    static constexpr const char* IFACE = "net.blackshark1.Headset";

    explicit HeadsetClient(QObject* parent = nullptr);

    // Properties.
    int battery() const;
    bool connected() const;
    QString eqPreset() const;
    QList<int> eqBands() const;
    int sidetone() const;
    bool noiseCancel() const;
    int sleepMinutes() const;
    bool powerSave() const;

    // Commands.
    void setEqPreset(const QString& name);
    void setEq(const QList<int>& bands);
    void setSidetone(int level);
    void setNoiseCancel(bool on);
    void setSleep(int minutes);
    void setPowerSave(bool on);
    void refreshBattery();
    void refreshSettings();
    QStringList eqProfiles();
    QList<int> eqProfile(const QString& name);
    void saveEqProfile(const QString& name, const QList<int>& bands);
    void deleteEqProfile(const QString& name);
    void applyEqProfile(const QString& name);

signals:
    // Emitted when battery or connection state changes (the tray and GUI listen).
    void batteryChanged(int percent);
    void connectedChanged(bool connected);
    void settingsChanged();
    void commandFailed(const QString& message);

private slots:
    void onPropertiesChanged(const QString& iface,
                             const QVariantMap& changed,
                             const QStringList& invalidated);

private:
    void invoke(const QString& method, const QVariantList& arguments = {});

    QDBusInterface m_iface;
    int m_battery = -1;
    bool m_connected = false;
    QString m_eqPreset = "default";
    QList<int> m_eqBands;
    int m_sidetone = 0;
    bool m_noiseCancel = false;
    int m_sleepMinutes = 0;
    bool m_powerSave = false;
};
