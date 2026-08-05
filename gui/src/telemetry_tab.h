#pragma once

#include <QWidget>
#include <QLabel>
#include <QPushButton>
#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QGridLayout>
#include <QJsonObject>
#include <QProgressBar>

class ApiClient;

class TelemetryTab : public QWidget {
    Q_OBJECT

public:
    explicit TelemetryTab(ApiClient *client, QWidget *parent = nullptr);
    ~TelemetryTab() override = default;

public slots:
    void refresh();

private slots:
    void onStatusReceived(const QJsonObject &data);

private:
    ApiClient *m_client;

    QLabel *m_chipNameLabel;
    QLabel *m_archLabel;
    QLabel *m_coresLabel;
    QLabel *m_ramLabel;
    QLabel *m_simdLabel;

    QLabel *m_modelParamsLabel;
    QLabel *m_vocabSizeLabel;
    QLabel *m_modelPathLabel;
    QLabel *m_engineLabel;

    QLabel *m_serverStatusLabel;
    QLabel *m_serverUrlLabel;
};

