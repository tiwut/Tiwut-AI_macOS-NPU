#include "api_client.h"
#include <QDebug>

ApiClient::ApiClient(QObject *parent)
    : QObject(parent)
    , m_baseUrl("http://127.0.0.1:8080")
    , m_nam(new QNetworkAccessManager(this))
    , m_healthTimer(new QTimer(this))
{
    connect(m_healthTimer, &QTimer::timeout, this, &ApiClient::checkHealth);
    m_healthTimer->start(3000);
}

void ApiClient::setBaseUrl(const QString &url) {
    m_baseUrl = url.trimmed();
    if (m_baseUrl.endsWith('/')) {
        m_baseUrl.chop(1);
    }
    checkHealth();
}

void ApiClient::checkHealth() {
    QUrl url(m_baseUrl + "/api/health");
    QNetworkRequest req(url);
    req.setHeader(QNetworkRequest::ContentTypeHeader, "application/json");

    QNetworkReply *reply = m_nam->get(req);
    connect(reply, &QNetworkReply::finished, this, &ApiClient::onHealthReply);
}

void ApiClient::onHealthReply() {
    auto *reply = qobject_cast<QNetworkReply *>(sender());
    if (!reply) return;

    if (reply->error() == QNetworkReply::NoError) {
        emit healthStatusChanged(true, "Connected to Tiwut-AI API");
    } else {
        emit healthStatusChanged(false, QString("Disconnected (%1)").arg(reply->errorString()));
    }
    reply->deleteLater();
}

void ApiClient::fetchStatus() {
    QUrl url(m_baseUrl + "/api/status");
    QNetworkRequest req(url);
    QNetworkReply *reply = m_nam->get(req);
    connect(reply, &QNetworkReply::finished, this, &ApiClient::onStatusReply);
}

void ApiClient::onStatusReply() {
    auto *reply = qobject_cast<QNetworkReply *>(sender());
    if (!reply) return;

    if (reply->error() == QNetworkReply::NoError) {
        QByteArray data = reply->readAll();
        QJsonDocument doc = QJsonDocument::fromJson(data);
        if (doc.isObject()) {
            emit statusReceived(doc.object());
        }
    }
    reply->deleteLater();
}

void ApiClient::fetchMemory() {
    QUrl url(m_baseUrl + "/api/memory");
    QNetworkRequest req(url);
    QNetworkReply *reply = m_nam->get(req);
    connect(reply, &QNetworkReply::finished, this, &ApiClient::onMemoryReply);
}

void ApiClient::onMemoryReply() {
    auto *reply = qobject_cast<QNetworkReply *>(sender());
    if (!reply) return;

    if (reply->error() == QNetworkReply::NoError) {
        QByteArray data = reply->readAll();
        QJsonDocument doc = QJsonDocument::fromJson(data);
        if (doc.isObject()) {
            emit memoryReceived(doc.object());
        }
    }
    reply->deleteLater();
}

void ApiClient::fetchConfig() {
    QUrl url(m_baseUrl + "/api/config");
    QNetworkRequest req(url);
    QNetworkReply *reply = m_nam->get(req);
    connect(reply, &QNetworkReply::finished, this, &ApiClient::onConfigReply);
}

void ApiClient::onConfigReply() {
    auto *reply = qobject_cast<QNetworkReply *>(sender());
    if (!reply) return;

    if (reply->error() == QNetworkReply::NoError) {
        QByteArray data = reply->readAll();
        QJsonDocument doc = QJsonDocument::fromJson(data);
        if (doc.isObject()) {
            emit configReceived(doc.object());
        }
    }
    reply->deleteLater();
}

void ApiClient::updateConfig(const QJsonObject &config) {
    QUrl url(m_baseUrl + "/api/config");
    QNetworkRequest req(url);
    req.setHeader(QNetworkRequest::ContentTypeHeader, "application/json");

    QByteArray body = QJsonDocument(config).toJson();
    QNetworkReply *reply = m_nam->post(req, body);
    connect(reply, &QNetworkReply::finished, this, &ApiClient::onConfigUpdateReply);
}

void ApiClient::onConfigUpdateReply() {
    auto *reply = qobject_cast<QNetworkReply *>(sender());
    if (!reply) return;

    bool ok = (reply->error() == QNetworkReply::NoError);
    emit configSaved(ok);
    reply->deleteLater();
}

void ApiClient::resetModel() {
    QUrl url(m_baseUrl + "/api/reset");
    QNetworkRequest req(url);
    req.setHeader(QNetworkRequest::ContentTypeHeader, "application/json");

    QNetworkReply *reply = m_nam->post(req, QByteArray("{}"));
    connect(reply, &QNetworkReply::finished, this, &ApiClient::onResetReply);
}

void ApiClient::onResetReply() {
    auto *reply = qobject_cast<QNetworkReply *>(sender());
    if (!reply) return;

    bool ok = (reply->error() == QNetworkReply::NoError);
    emit modelResetCompleted(ok);
    reply->deleteLater();
}

void ApiClient::sendChat(const QString &message, double temp, int maxTokens) {
    QUrl url(m_baseUrl + "/api/chat");
    QNetworkRequest req(url);
    req.setHeader(QNetworkRequest::ContentTypeHeader, "application/json");

    QJsonObject obj;
    obj["message"] = message;
    obj["temperature"] = temp;
    obj["max_tokens"] = maxTokens;

    QByteArray body = QJsonDocument(obj).toJson();
    QNetworkReply *reply = m_nam->post(req, body);
    connect(reply, &QNetworkReply::finished, this, &ApiClient::onChatReply);
}

void ApiClient::onChatReply() {
    auto *reply = qobject_cast<QNetworkReply *>(sender());
    if (!reply) return;

    if (reply->error() == QNetworkReply::NoError) {
        QByteArray data = reply->readAll();
        QJsonDocument doc = QJsonDocument::fromJson(data);
        if (doc.isObject()) {
            QString resp = doc.object()["response"].toString();
            emit chatChunkReceived(resp);
            emit chatFinished();
        }
    } else {
        emit chatError(reply->errorString());
    }
    reply->deleteLater();
}

void ApiClient::sendChatStream(const QString &message, double temp, int maxTokens) {
    abortChatStream();

    QUrl url(m_baseUrl + "/api/chat/stream");
    QNetworkRequest req(url);
    req.setHeader(QNetworkRequest::ContentTypeHeader, "application/json");
    req.setAttribute(QNetworkRequest::Http2DirectAttribute, true);

    QJsonObject obj;
    obj["message"] = message;
    obj["temperature"] = temp;
    obj["max_tokens"] = maxTokens;

    QByteArray body = QJsonDocument(obj).toJson();
    m_activeStreamReply = m_nam->post(req, body);

    connect(m_activeStreamReply, &QNetworkReply::readyRead, this, &ApiClient::onChatStreamReadyRead);
    connect(m_activeStreamReply, &QNetworkReply::finished, this, &ApiClient::onChatStreamFinished);
}

void ApiClient::abortChatStream() {
    if (m_activeStreamReply) {
        m_activeStreamReply->abort();
        m_activeStreamReply->deleteLater();
        m_activeStreamReply = nullptr;
    }
}

void ApiClient::onChatStreamReadyRead() {
    if (!m_activeStreamReply) return;

    while (m_activeStreamReply->canReadLine()) {
        QByteArray line = m_activeStreamReply->readLine();
        while (!line.isEmpty() && (line.endsWith('\r') || line.endsWith('\n'))) {
            line.chop(1);
        }

        if (line.startsWith("data:")) {
            QByteArray payloadBytes = line.mid(5);
            if (payloadBytes.startsWith(' ')) {
                payloadBytes = payloadBytes.mid(1);
            }

            QString payload = QString::fromUtf8(payloadBytes);
            if (payload == "[DONE]") {
                emit chatFinished();
            } else if (payload.isEmpty()) {
                emit chatChunkReceived("\n");
            } else {
                emit chatChunkReceived(payload);
            }
        }
    }
}

void ApiClient::onChatStreamFinished() {
    if (m_activeStreamReply) {
        if (m_activeStreamReply->error() != QNetworkReply::NoError && m_activeStreamReply->error() != QNetworkReply::OperationCanceledError) {
            emit chatError(m_activeStreamReply->errorString());
        } else {
            emit chatFinished();
        }
        m_activeStreamReply->deleteLater();
        m_activeStreamReply = nullptr;
    }
}

void ApiClient::askQuestion(const QString &question) {
    QUrl url(m_baseUrl + "/api/ask");
    QNetworkRequest req(url);
    req.setHeader(QNetworkRequest::ContentTypeHeader, "application/json");

    QJsonObject obj;
    obj["question"] = question;

    QByteArray body = QJsonDocument(obj).toJson();
    QNetworkReply *reply = m_nam->post(req, body);
    connect(reply, &QNetworkReply::finished, this, &ApiClient::onAskReply);
}

void ApiClient::onAskReply() {
    auto *reply = qobject_cast<QNetworkReply *>(sender());
    if (!reply) return;

    if (reply->error() == QNetworkReply::NoError) {
        QByteArray data = reply->readAll();
        QJsonDocument doc = QJsonDocument::fromJson(data);
        if (doc.isObject()) {
            QString q = doc.object()["question"].toString();
            QString a = doc.object()["answer"].toString();
            emit askAnswerReceived(q, a);
        }
    } else {
        emit askError(reply->errorString());
    }
    reply->deleteLater();
}

void ApiClient::startTraining(const QStringList &urls, const QStringList &files, const QString &text, int epochs, double lr, bool includeDefault) {
    QUrl url(m_baseUrl + "/api/train");
    QNetworkRequest req(url);
    req.setHeader(QNetworkRequest::ContentTypeHeader, "application/json");

    QJsonObject obj;
    if (!urls.isEmpty()) {
        QJsonArray arr;
        for (const auto &u : urls) arr.append(u);
        obj["urls"] = arr;
    }
    if (!files.isEmpty()) {
        QJsonArray arr;
        for (const auto &f : files) arr.append(f);
        obj["files"] = arr;
    }
    if (!text.isEmpty()) {
        obj["text"] = text;
    }
    obj["epochs"] = epochs;
    obj["learning_rate"] = lr;
    obj["include_default_knowledge"] = includeDefault;

    QByteArray body = QJsonDocument(obj).toJson();
    QNetworkReply *reply = m_nam->post(req, body);
    connect(reply, &QNetworkReply::finished, this, &ApiClient::onTrainReply);
}

void ApiClient::onTrainReply() {
    auto *reply = qobject_cast<QNetworkReply *>(sender());
    if (!reply) return;

    if (reply->error() == QNetworkReply::NoError) {
        QByteArray data = reply->readAll();
        QJsonDocument doc = QJsonDocument::fromJson(data);
        if (doc.isObject()) {
            QString msg = doc.object()["message"].toString();
            emit trainingFinished(true, msg);
        } else {
            emit trainingFinished(true, "Training finished successfully.");
        }
    } else {
        emit trainingFinished(false, QString("Training error: %1").arg(reply->errorString()));
    }
    reply->deleteLater();
}

