#pragma once

#include <QMainWindow>
#include <QStackedWidget>
#include <QPushButton>
#include <QLabel>
#include <QButtonGroup>

class ApiClient;
class ChatTab;
class TrainTab;
class MemoryTab;
class TelemetryTab;
class ConfigTab;

class MainWindow : public QMainWindow {
    Q_OBJECT

public:
    explicit MainWindow(QWidget *parent = nullptr);
    ~MainWindow() override = default;

private slots:
    void onNavButtonClicked(int id);
    void onHealthStatusChanged(bool connected, const QString &info);

private:
    void setupUi();
    void setupSidebar();

    ApiClient *m_client;

    QStackedWidget *m_stackedWidget;
    QButtonGroup *m_navGroup;

    QPushButton *m_chatNavBtn;
    QPushButton *m_trainNavBtn;
    QPushButton *m_memoryNavBtn;
    QPushButton *m_telemetryNavBtn;
    QPushButton *m_configNavBtn;

    ChatTab *m_chatTab;
    TrainTab *m_trainTab;
    MemoryTab *m_memoryTab;
    TelemetryTab *m_telemetryTab;
    ConfigTab *m_configTab;

    QLabel *m_statusDot;
    QLabel *m_statusText;
};

