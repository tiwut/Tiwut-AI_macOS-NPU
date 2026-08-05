cmake_minimum_required(VERSION 3.16)
project(tiwut-ai-gui LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTORCC ON)
set(CMAKE_AUTOUIC ON)

# Ensure Qt6 is found on macOS Homebrew and standard Linux/Windows paths
list(APPEND CMAKE_PREFIX_PATH 
    "/opt/homebrew" 
    "/opt/homebrew/lib/cmake/Qt6"
    "/opt/homebrew/opt/qt" 
    "/opt/homebrew/opt/qt@6"
    "/usr/local/opt/qt"
    "/usr/local/opt/qt@6"
)

find_package(Qt6 REQUIRED COMPONENTS Core Gui Widgets Network)

set(SOURCES
    src/main.cpp
    src/api_client.h
    src/api_client.cpp
    src/chat_tab.h
    src/chat_tab.cpp
    src/train_tab.h
    src/train_tab.cpp
    src/memory_tab.h
    src/memory_tab.cpp
    src/telemetry_tab.h
    src/telemetry_tab.cpp
    src/config_tab.h
    src/config_tab.cpp
    src/main_window.h
    src/main_window.cpp
)

set(RESOURCES
    resources/resources.qrc
)

add_executable(tiwut-ai-gui
    ${SOURCES}
    ${RESOURCES}
)

target_link_libraries(tiwut-ai-gui PRIVATE
    Qt6::Core
    Qt6::Gui
    Qt6::Widgets
    Qt6::Network
)

if(APPLE)
    set_target_properties(tiwut-ai-gui PROPERTIES
        MACOSX_BUNDLE FALSE
    )
endif()
