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

    // Commands.
    void setEqPreset(const QString& name);
    void setEq(const QList<int>& bands);
    void setSidetone(int level);
    void setNoiseCancel(bool on);
    void setSleep(int minutes);
    void setPowerSave(bool on);
    void refreshBattery();

signals:
    // Emitted when battery or connection state changes (the tray and GUI listen).
    void batteryChanged(int percent);
    void connectedChanged(bool connected);

private slots:
    void onPropertiesChanged(const QString& iface,
                             const QVariantMap& changed,
                             const QStringList& invalidated);

private:
    QDBusInterface m_iface;
    int m_battery = -1;
    bool m_connected = false;
};
