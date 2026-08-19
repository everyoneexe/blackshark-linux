#include "HeadsetClient.h"

#include <QtDBus/QDBusArgument>
#include <QtDBus/QDBusMessage>
#include <QtDBus/QDBusReply>
#include <QTimer>
#include <QVariant>

HeadsetClient::HeadsetClient(QObject* parent)
    : QObject(parent),
      m_iface(SERVICE, PATH, IFACE, QDBusConnection::sessionBus(), this) {
    // Listen for the PropertiesChanged signal (org.freedesktop.DBus.Properties).
    QDBusConnection::sessionBus().connect(
        SERVICE, PATH, "org.freedesktop.DBus.Properties",
        "PropertiesChanged", this,
        SLOT(onPropertiesChanged(QString, QVariantMap, QStringList)));

    // Read the initial values and keep the UI in sync with settings changed by
    // another client (for example, the CLI).
    QVariant b = m_iface.property("Battery");
    if (b.isValid()) m_battery = b.toInt();
    QVariant c = m_iface.property("Connected");
    if (c.isValid()) m_connected = c.toBool();
    refreshSettings();

    auto* refreshTimer = new QTimer(this);
    connect(refreshTimer, &QTimer::timeout, this, &HeadsetClient::refreshSettings);
    refreshTimer->start(3000);
}

int HeadsetClient::battery() const { return m_battery; }
bool HeadsetClient::connected() const { return m_connected; }
QString HeadsetClient::eqPreset() const { return m_eqPreset; }
QList<int> HeadsetClient::eqBands() const { return m_eqBands; }
int HeadsetClient::sidetone() const { return m_sidetone; }
bool HeadsetClient::noiseCancel() const { return m_noiseCancel; }
int HeadsetClient::sleepMinutes() const { return m_sleepMinutes; }
bool HeadsetClient::powerSave() const { return m_powerSave; }

void HeadsetClient::setEqPreset(const QString& name) {
    invoke("SetEqPreset", {name});
}

void HeadsetClient::setEq(const QList<int>& bands) {
    // QDBus expects the daemon's exact `ai` signature. A QVariantList would be
    // serialized as `av` and is rejected by the Rust D-Bus method.
    QDBusArgument dbusBands;
    dbusBands.beginArray(qMetaTypeId<int>());
    for (int band : bands) dbusBands << band;
    dbusBands.endArray();
    invoke("SetEq", {QVariant::fromValue(dbusBands)});
}

QStringList HeadsetClient::eqProfiles() {
    QDBusReply<QStringList> reply = m_iface.call("ListEqProfiles");
    if (!reply.isValid()) {
        emit commandFailed(QString("could not list EQ profiles: %1").arg(reply.error().message()));
        return {};
    }
    return reply.value();
}

QList<int> HeadsetClient::eqProfile(const QString& name) {
    QDBusReply<QString> reply = m_iface.call("GetEqProfile", name);
    if (!reply.isValid()) {
        emit commandFailed(QString("could not load EQ profile: %1").arg(reply.error().message()));
        return {};
    }
    QList<int> bands;
    for (const QString& band : reply.value().split(',', Qt::SkipEmptyParts)) {
        bands.append(band.toInt());
    }
    return bands;
}

void HeadsetClient::saveEqProfile(const QString& name, const QList<int>& bands) {
    QDBusArgument dbusBands;
    dbusBands.beginArray(qMetaTypeId<int>());
    for (int band : bands) dbusBands << band;
    dbusBands.endArray();
    invoke("SaveEqProfile", {name, QVariant::fromValue(dbusBands)});
}

void HeadsetClient::deleteEqProfile(const QString& name) {
    invoke("DeleteEqProfile", {name});
}

void HeadsetClient::applyEqProfile(const QString& name) {
    invoke("ApplyEqProfile", {name});
}

void HeadsetClient::setSidetone(int level) {
    invoke("SetSidetone", {(uint)level});
}

void HeadsetClient::setNoiseCancel(bool on) {
    invoke("SetNoiseCancel", {on});
}

void HeadsetClient::setSleep(int minutes) {
    invoke("SetSleep", {(uint)minutes});
}

void HeadsetClient::setPowerSave(bool on) {
    invoke("SetPowerSave", {on});
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

void HeadsetClient::refreshSettings() {
    const QVariant eqPreset = m_iface.property("EqPreset");
    const QVariant eqBands = m_iface.property("EqBands");
    const QVariant sidetone = m_iface.property("Sidetone");
    const QVariant noiseCancel = m_iface.property("NoiseCancel");
    const QVariant sleep = m_iface.property("Sleep");
    const QVariant powerSave = m_iface.property("PowerSave");

    bool changed = false;
    if (eqPreset.isValid() && eqPreset.toString() != m_eqPreset) {
        m_eqPreset = eqPreset.toString();
        changed = true;
    }
    if (eqBands.isValid()) {
        QList<int> bands;
        for (const QString& band : eqBands.toString().split(',', Qt::SkipEmptyParts)) {
            bands.append(band.toInt());
        }
        if (bands != m_eqBands) {
            m_eqBands = bands;
            changed = true;
        }
    }
    if (sidetone.isValid() && sidetone.toInt() != m_sidetone) {
        m_sidetone = sidetone.toInt();
        changed = true;
    }
    if (noiseCancel.isValid() && noiseCancel.toBool() != m_noiseCancel) {
        m_noiseCancel = noiseCancel.toBool();
        changed = true;
    }
    if (sleep.isValid() && sleep.toInt() != m_sleepMinutes) {
        m_sleepMinutes = sleep.toInt();
        changed = true;
    }
    if (powerSave.isValid() && powerSave.toBool() != m_powerSave) {
        m_powerSave = powerSave.toBool();
        changed = true;
    }
    if (changed) emit settingsChanged();
}

void HeadsetClient::invoke(const QString& method, const QVariantList& arguments) {
    const QDBusMessage reply = m_iface.callWithArgumentList(QDBus::Block, method, arguments);
    if (reply.type() == QDBusMessage::ErrorMessage) {
        emit commandFailed(QString("%1 failed: %2").arg(method, reply.errorMessage()));
        return;
    }
    refreshSettings();
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
