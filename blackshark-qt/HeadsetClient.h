#pragma once

#include <QObject>
#include <QString>
#include <QtDBus/QDBusInterface>
#include <QtDBus/QDBusConnection>

// Daemon (net.blackshark1) icin D-Bus istemcisi.
// Tray ve GUI bunu paylasir. HID'e dokunmaz; her sey daemon uzerinden gider.
class HeadsetClient : public QObject {
    Q_OBJECT
public:
    static constexpr const char* SERVICE = "net.blackshark1";
    static constexpr const char* PATH = "/net/blackshark1/Headset";
    static constexpr const char* IFACE = "net.blackshark1.Headset";

    explicit HeadsetClient(QObject* parent = nullptr);

    // Property'ler
    int battery() const;
    bool connected() const;

    // Komutlar
    void setEqPreset(const QString& name);
    void setEq(const QList<int>& bands);
    void setSidetone(int level);
    void setNoiseCancel(bool on);
    void setSleep(int minutes);
    void setPowerSave(bool on);
    void refreshBattery();

signals:
    // Batarya/baglanti degisince yayilir (tray ve GUI dinler).
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
