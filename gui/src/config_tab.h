#pragma once

#include <QWidget>
#include <QLineEdit>
#include <QSpinBox>
#include <QDoubleSpinBox>
#include <QSlider>
#include <QPushButton>
#include <QLabel>
#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QJsonObject>

class ApiClient;

class ConfigTab : public QWidget {
    Q_OBJECT

public:
    explicit ConfigTab(ApiClient *client, QWidget *parent = nullptr);
    ~ConfigTab() override = default;

public slots:
    void refresh();

private slots:
    void onSaveApiSettings();
    void onSaveConfig();
    void onResetModel();
    void onConfigReceived(const QJsonObject &data);
    void onConfigSaved(bool success);
    void onModelResetCompleted(bool success);

private:
    ApiClient *m_client;

    QLineEdit *m_apiUrlInput;
    QPushButton *m_saveApiBtn;

    QDoubleSpinBox *m_tempSpin;
    QSpinBox *m_topKSpin;
    QDoubleSpinBox *m_topPSpin;
    QDoubleSpinBox *m_repPenaltySpin;
    QSpinBox *m_maxTokensSpin;
    QDoubleSpinBox *m_memThresholdSpin;

    QPushButton *m_saveConfigBtn;
    QPushButton *m_resetModelBtn;
    QLabel *m_statusMsgLabel;

    QJsonObject m_currentConfig;
};

