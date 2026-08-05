#pragma once

#include <QObject>
#include <QString>
#include <QStringList>
#include <QJsonObject>
#include <QJsonDocument>
#include <QJsonArray>
#include <QNetworkAccessManager>
#include <QNetworkReply>
#include <QNetworkRequest>
#include <QUrl>
#include <QTimer>

class ApiClient : public QObject {
    Q_OBJECT

public:
    explicit ApiClient(QObject *parent = nullptr);
    ~ApiClient() override = default;

    void setBaseUrl(const QString &url);
    QString baseUrl() const { return m_baseUrl; }

    void checkHealth();
    void fetchStatus();
    void fetchMemory();
    void fetchConfig();
    void updateConfig(const QJsonObject &config);
    void resetModel();

    void sendChat(const QString &message, double temp = 0.7, int maxTokens = 256);
    void sendChatStream(const QString &message, double temp = 0.7, int maxTokens = 256);
    void abortChatStream();

    void askQuestion(const QString &question);
    void startTraining(const QStringList &urls, const QStringList &files, const QString &text, int epochs = 8, double lr = 0.0004, bool includeDefault = false);

signals:
    void healthStatusChanged(bool connected, const QString &info);
    void statusReceived(const QJsonObject &data);
    void memoryReceived(const QJsonObject &data);
    void configReceived(const QJsonObject &data);
    void configSaved(bool success);
    void modelResetCompleted(bool success);

    void chatChunkReceived(const QString &token);
    void chatFinished();
    void chatError(const QString &error);

    void askAnswerReceived(const QString &question, const QString &answer);
    void askError(const QString &error);

    void trainingProgress(const QString &log);
    void trainingFinished(bool success, const QString &message);

private slots:
    void onHealthReply();
    void onStatusReply();
    void onMemoryReply();
    void onConfigReply();
    void onConfigUpdateReply();
    void onResetReply();
    void onChatReply();
    void onChatStreamReadyRead();
    void onChatStreamFinished();
    void onAskReply();
    void onTrainReply();

private:
    QString m_baseUrl;
    QNetworkAccessManager *m_nam;
    QNetworkReply *m_activeStreamReply{nullptr};
    QTimer *m_healthTimer{nullptr};
};

