cmake_minimum_required(VERSION 3.16)
project(blackshark-qt LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTORCC ON)

# Prefer Qt6; fall back to Qt5.
find_package(Qt6 COMPONENTS Widgets DBus Svg QUIET)
if (Qt6_FOUND)
    set(QT_LIBS Qt6::Widgets Qt6::DBus Qt6::Svg)
else()
    find_package(Qt5 5.12 REQUIRED COMPONENTS Widgets DBus Svg)
    set(QT_LIBS Qt5::Widgets Qt5::DBus Qt5::Svg)
endif()

add_executable(blackshark-tray
    main.cpp
    HeadsetClient.cpp
    TrayApp.cpp
    EqDialog.cpp
    resources.qrc
)

target_link_libraries(blackshark-tray PRIVATE ${QT_LIBS})

install(TARGETS blackshark-tray RUNTIME DESTINATION bin)
