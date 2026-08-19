#include "HeadsetClient.h"

#include <QtDBus/QDBusReply>
#include <QVariant>

HeadsetClient::HeadsetClient(QObject* parent)
    : QObject(parent),
      m_iface(SERVICE, PATH, IFACE, QDBusConnection::sessionBus(), this) {
    // PropertiesChanged sinyalini dinle (org.freedesktop.DBus.Properties).
    QDBusConnection::sessionBus().connect(
        SERVICE, PATH, "org.freedesktop.DBus.Properties",
        "PropertiesChanged", this,
        SLOT(onPropertiesChanged(QString, QVariantMap, QStringList)));

    // Ilk degerleri cek.
    QVariant b = m_iface.property("Battery");
    if (b.isValid()) m_battery = b.toInt();
    QVariant c = m_iface.property("Connected");
    if (c.isValid()) m_connected = c.toBool();
}

int HeadsetClient::battery() const { return m_battery; }
bool HeadsetClient::connected() const { return m_connected; }

void HeadsetClient::setEqPreset(const QString& name) {
    m_iface.call("SetEqPreset", name);
}

void HeadsetClient::setEq(const QList<int>& bands) {
    QList<QVariant> vlist;
    // D-Bus imzasi ai (array of int32)
    QVariantList arr;
    for (int b : bands) arr << b;
    m_iface.call("SetEq", QVariant::fromValue(arr));
}

void HeadsetClient::setSidetone(int level) {
    m_iface.call("SetSidetone", (uint)level);
}

void HeadsetClient::setNoiseCancel(bool on) {
    m_iface.call("SetNoiseCancel", on);
}

void HeadsetClient::setSleep(int minutes) {
    m_iface.call("SetSleep", (uint)minutes);
}

void HeadsetClient::setPowerSave(bool on) {
    m_iface.call("SetPowerSave", on);
}

void HeadsetClient::refreshBattery() {
    QDBusReply<int> reply = m_iface.call("RefreshBattery");
    if (reply.isValid()) {
        int v = reply.value();
        if (v != m_battery) {
            m_battery = v;
            emit batteryChanged(v);
        }
    }
}

void HeadsetClient::onPropertiesChanged(const QString& iface,
                                        const QVariantMap& changed,
                                        const QStringList&) {
    if (iface != IFACE) return;
    if (changed.contains("Battery")) {
        m_battery = changed.value("Battery").toInt();
        emit batteryChanged(m_battery);
    }
    if (changed.contains("Connected")) {
        m_connected = changed.value("Connected").toBool();
        emit connectedChanged(m_connected);
    }
}
