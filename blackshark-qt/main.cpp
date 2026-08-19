#include <QApplication>
#include <QSystemTrayIcon>
#include <QMessageBox>

#include "HeadsetClient.h"
#include "TrayApp.h"

int main(int argc, char* argv[]) {
    QApplication app(argc, argv);
    app.setApplicationName("BlackShark Tray");
    app.setQuitOnLastWindowClosed(false); // keep running when the settings window closes

    if (!QSystemTrayIcon::isSystemTrayAvailable()) {
        QMessageBox::critical(nullptr, "BlackShark",
                              "System tray is not available.");
        return 1;
    }

    auto* client = new HeadsetClient(&app);
    auto* tray = new TrayApp(client, &app);
    Q_UNUSED(tray);

    return app.exec();
}
