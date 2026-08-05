#pragma once

#include <QWidget>
#include <QScrollArea>
#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QTextEdit>
#include <QLineEdit>
#include <QPushButton>
#include <QLabel>
#include <QFrame>
#include <QString>

class ApiClient;

class MessageCard : public QFrame {
    Q_OBJECT

public:
    enum Role { User, Assistant, System };

    explicit MessageCard(Role role, const QString &text, QWidget *parent = nullptr);
    void appendChunk(const QString &chunk);
    void finishStreaming();
    QString text() const { return m_fullText; }

private slots:
    void copyContent();

private:
    void updateRenderedContent();

    Role m_role;
    QString m_fullText;
    QLabel *m_headerLabel{nullptr};
    QLabel *m_bodyLabel{nullptr};
    QLabel *m_sourceBadge{nullptr};
    QPushButton *m_copyBtn{nullptr};
};

class ChatTab : public QWidget {
    Q_OBJECT

public:
    explicit ChatTab(ApiClient *client, QWidget *parent = nullptr);
    ~ChatTab() override = default;

private slots:
    void onSendClicked();
    void onStopClicked();
    void onClearClicked();
    void onQuickPrompt(const QString &text);

    void onChunkReceived(const QString &chunk);
    void onChatFinished();
    void onChatError(const QString &error);

private:
    void scrollToBottom();
    void addWelcomeBanner();

    ApiClient *m_client;
    QScrollArea *m_scrollArea;
    QWidget *m_chatContainer;
    QVBoxLayout *m_chatLayout;

    QTextEdit *m_inputEdit;
    QPushButton *m_sendBtn;
    QPushButton *m_stopBtn;
    QPushButton *m_clearBtn;
    QLabel *m_statusLabel;
    QLabel *m_tokenCounterLabel;

    MessageCard *m_currentAiCard{nullptr};
    bool m_isGenerating{false};
    int m_tokensReceived{0};
};

