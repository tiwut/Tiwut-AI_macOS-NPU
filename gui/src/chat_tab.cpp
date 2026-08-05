#include "chat_tab.h"
#include "api_client.h"
#include <QScrollBar>
#include <QKeyEvent>
#include <QClipboard>
#include <QApplication>
#include <QRegularExpression>
#include <QTimer>

class MessageInputEdit : public QTextEdit {
public:
    explicit MessageInputEdit(QWidget *parent = nullptr) : QTextEdit(parent) {}

    void setSendCallback(std::function<void()> cb) { m_sendCb = cb; }

protected:
    void keyPressEvent(QKeyEvent *e) override {
        if ((e->key() == Qt::Key_Return || e->key() == Qt::Key_Enter) && !(e->modifiers() & Qt::ShiftModifier)) {
            if (m_sendCb) m_sendCb();
            e->accept();
        } else {
            QTextEdit::keyPressEvent(e);
        }
    }

private:
    std::function<void()> m_sendCb;
};

MessageCard::MessageCard(Role role, const QString &text, QWidget *parent)
    : QFrame(parent)
    , m_role(role)
    , m_fullText(text)
{
    auto *cardLayout = new QVBoxLayout(this);
    cardLayout->setContentsMargins(14, 12, 14, 12);
    cardLayout->setSpacing(8);

    if (role == User) {

        setStyleSheet(
            "MessageCard {"
            "  background: qlineargradient(x1:0, y1:0, x2:1, y2:0, stop:0 #0284c7, stop:1 #2563eb);"
            "  border-radius: 14px;"
            "  border: 1px solid rgba(255, 255, 255, 0.15);"
            "}"
        );

        auto *topRow = new QHBoxLayout();
        auto *userLabel = new QLabel("👤 You", this);
        userLabel->setStyleSheet("color: #e0f2fe; font-weight: bold; font-size: 11px;");
        topRow->addWidget(userLabel);
        topRow->addStretch();
        cardLayout->addLayout(topRow);

        m_bodyLabel = new QLabel(this);
        m_bodyLabel->setWordWrap(true);
        m_bodyLabel->setTextInteractionFlags(Qt::TextSelectableByMouse);
        m_bodyLabel->setStyleSheet("color: #ffffff; font-size: 13.5px; line-height: 1.4;");
        cardLayout->addWidget(m_bodyLabel);

    } else if (role == Assistant) {

        setStyleSheet(
            "MessageCard {"
            "  background-color: rgba(30, 41, 59, 0.85);"
            "  border: 1px solid rgba(56, 189, 248, 0.25);"
            "  border-radius: 14px;"
            "}"
        );

        auto *topRow = new QHBoxLayout();
        m_headerLabel = new QLabel("🤖 Tiwut-AI", this);
        m_headerLabel->setStyleSheet("color: #38bdf8; font-weight: bold; font-size: 12px;");
        topRow->addWidget(m_headerLabel);

        topRow->addStretch();

        m_copyBtn = new QPushButton("📋 Copy", this);
        m_copyBtn->setStyleSheet(
            "QPushButton {"
            "  background: transparent;"
            "  color: #94a3b8;"
            "  border: none;"
            "  font-size: 11px;"
            "  padding: 2px 6px;"
            "}"
            "QPushButton:hover {"
            "  color: #38bdf8;"
            "  background: rgba(56, 189, 248, 0.1);"
            "  border-radius: 4px;"
            "}"
        );
        connect(m_copyBtn, &QPushButton::clicked, this, &MessageCard::copyContent);
        topRow->addWidget(m_copyBtn);
        cardLayout->addLayout(topRow);

        m_bodyLabel = new QLabel(this);
        m_bodyLabel->setWordWrap(true);
        m_bodyLabel->setTextInteractionFlags(Qt::TextSelectableByMouse | Qt::LinksAccessibleByMouse);
        m_bodyLabel->setStyleSheet("color: #f1f5f9; font-size: 13.5px; line-height: 1.5;");
        cardLayout->addWidget(m_bodyLabel);

        m_sourceBadge = new QLabel(this);
        m_sourceBadge->setWordWrap(true);
        m_sourceBadge->setStyleSheet(
            "QLabel {"
            "  background-color: rgba(15, 23, 42, 0.7);"
            "  color: #38bdf8;"
            "  border: 1px solid rgba(56, 189, 248, 0.3);"
            "  border-radius: 6px;"
            "  padding: 5px 8px;"
            "  font-size: 11.5px;"
            "}"
        );
        m_sourceBadge->setVisible(false);
        cardLayout->addWidget(m_sourceBadge);

    } else {

        setStyleSheet(
            "MessageCard {"
            "  background-color: rgba(15, 23, 42, 0.6);"
            "  border: 1px dashed rgba(255, 255, 255, 0.1);"
            "  border-radius: 10px;"
            "}"
        );
        m_bodyLabel = new QLabel(this);
        m_bodyLabel->setWordWrap(true);
        m_bodyLabel->setStyleSheet("color: #94a3b8; font-size: 12px;");
        cardLayout->addWidget(m_bodyLabel);
    }

    updateRenderedContent();
}

void MessageCard::appendChunk(const QString &chunk) {
    m_fullText += chunk;
    updateRenderedContent();
}

void MessageCard::finishStreaming() {
    updateRenderedContent();
}

void MessageCard::copyContent() {
    QClipboard *clipboard = QApplication::clipboard();
    clipboard->setText(m_fullText);
    if (m_copyBtn) {
        m_copyBtn->setText("✅ Copied!");
        QTimer::singleShot(2000, this, [this]() {
            if (m_copyBtn) m_copyBtn->setText("📋 Copy");
        });
    }
}

void MessageCard::updateRenderedContent() {
    if (!m_bodyLabel) return;

    QString text = m_fullText;

    static QRegularExpression sourceRegex(R"(\[Source:\s*([^\]]+)\])");
    auto match = sourceRegex.match(text);
    if (match.hasMatch() && m_sourceBadge) {
        QString sourcePath = match.captured(1).trimmed();
        m_sourceBadge->setText(QString("🔗 <b>Source:</b> %1").arg(sourcePath.toHtmlEscaped()));
        m_sourceBadge->setVisible(true);
        text.remove(match.capturedStart(0), match.capturedLength(0));
        text = text.trimmed();
    }

    QString escaped = text.toHtmlEscaped();

    escaped.replace("\n\n", "<br/><br/>");
    escaped.replace("\n", "<br/>");

    static QRegularExpression boldRegex(R"(\*\*(.*?)\*\*)");
    escaped.replace(boldRegex, "<b>\\1</b>");

    static QRegularExpression codeRegex(R"(`([^`]+)`)");
    escaped.replace(codeRegex, "<span style='background:#0f172a; color:#38bdf8; padding:2px 5px; border-radius:4px; font-family:monospace;'>\\1</span>");

    m_bodyLabel->setText(escaped);
}

ChatTab::ChatTab(ApiClient *client, QWidget *parent)
    : QWidget(parent)
    , m_client(client)
{
    auto *mainLayout = new QVBoxLayout(this);
    mainLayout->setContentsMargins(20, 20, 20, 20);
    mainLayout->setSpacing(12);

    auto *headerLayout = new QHBoxLayout();
    auto *title = new QLabel("💬 Interactive Neural Chat Studio", this);
    title->setProperty("class", "sectionHeader");
    headerLayout->addWidget(title);

    headerLayout->addStretch();

    m_tokenCounterLabel = new QLabel("Tokens: 0", this);
    m_tokenCounterLabel->setStyleSheet("color: #94a3b8; font-weight: 500; font-size: 12px;");
    headerLayout->addWidget(m_tokenCounterLabel);

    m_clearBtn = new QPushButton("Clear History", this);
    m_clearBtn->setProperty("class", "secondaryBtn");
    connect(m_clearBtn, &QPushButton::clicked, this, &ChatTab::onClearClicked);
    headerLayout->addWidget(m_clearBtn);

    mainLayout->addLayout(headerLayout);

    m_scrollArea = new QScrollArea(this);
    m_scrollArea->setWidgetResizable(true);
    m_scrollArea->setFrameShape(QFrame::NoFrame);
    m_scrollArea->setStyleSheet(
        "QScrollArea {"
        "  background-color: #080d1a;"
        "  border: 1px solid rgba(255, 255, 255, 0.08);"
        "  border-radius: 12px;"
        "}"
    );

    m_chatContainer = new QWidget();
    m_chatContainer->setStyleSheet("background-color: transparent;");
    m_chatLayout = new QVBoxLayout(m_chatContainer);
    m_chatLayout->setContentsMargins(16, 16, 16, 16);
    m_chatLayout->setSpacing(14);
    m_chatLayout->addStretch();

    m_scrollArea->setWidget(m_chatContainer);
    mainLayout->addWidget(m_scrollArea, 1);

    addWelcomeBanner();

    auto *quickLayout = new QHBoxLayout();
    quickLayout->setSpacing(8);

    QStringList quickPrompts = {
        "What is Tiwut-AI?",
        "What is Apple Silicon?",
        "What is Rust?",
        "How does RAG work?",
        "What is an NPU?"
    };

    for (const auto &prompt : quickPrompts) {
        auto *btn = new QPushButton(prompt, this);
        btn->setStyleSheet(
            "QPushButton {"
            "  background-color: rgba(56, 189, 248, 0.1);"
            "  color: #38bdf8;"
            "  border: 1px solid rgba(56, 189, 248, 0.25);"
            "  border-radius: 6px;"
            "  padding: 5px 10px;"
            "  font-size: 11.5px;"
            "}"
            "QPushButton:hover {"
            "  background-color: rgba(56, 189, 248, 0.2);"
            "  border-color: #38bdf8;"
            "}"
        );
        connect(btn, &QPushButton::clicked, this, [this, prompt]() {
            onQuickPrompt(prompt);
        });
        quickLayout->addWidget(btn);
    }
    quickLayout->addStretch();
    mainLayout->addLayout(quickLayout);

    auto *inputContainer = new QHBoxLayout();
    inputContainer->setSpacing(10);

    auto *customInput = new MessageInputEdit(this);
    customInput->setPlaceholderText("Ask a question, discuss code, or explore ideas... (Press Enter to send, Shift+Enter for newline)");
    customInput->setFixedHeight(62);
    customInput->setSendCallback([this]() { onSendClicked(); });
    m_inputEdit = customInput;
    inputContainer->addWidget(m_inputEdit, 1);

    auto *btnCol = new QVBoxLayout();
    btnCol->setSpacing(6);

    m_sendBtn = new QPushButton("Send 🚀", this);
    m_sendBtn->setProperty("class", "primaryBtn");
    m_sendBtn->setFixedHeight(32);
    connect(m_sendBtn, &QPushButton::clicked, this, &ChatTab::onSendClicked);
    btnCol->addWidget(m_sendBtn);

    m_stopBtn = new QPushButton("Stop ⏹️", this);
    m_stopBtn->setProperty("class", "dangerBtn");
    m_stopBtn->setFixedHeight(26);
    m_stopBtn->setEnabled(false);
    connect(m_stopBtn, &QPushButton::clicked, this, &ChatTab::onStopClicked);
    btnCol->addWidget(m_stopBtn);

    inputContainer->addLayout(btnCol);
    mainLayout->addLayout(inputContainer);

    m_statusLabel = new QLabel("Ready • Standalone API Connected", this);
    m_statusLabel->setStyleSheet("color: #64748b; font-size: 11px;");
    mainLayout->addWidget(m_statusLabel);

    connect(m_client, &ApiClient::chatChunkReceived, this, &ChatTab::onChunkReceived);
    connect(m_client, &ApiClient::chatFinished, this, &ChatTab::onChatFinished);
    connect(m_client, &ApiClient::chatError, this, &ChatTab::onChatError);
}

void ChatTab::addWelcomeBanner() {
    auto *banner = new MessageCard(
        MessageCard::System,
        "✨ <b>Welcome to Tiwut-AI v2 Neural Chat Studio</b><br/>"
        "Tiwut-AI is running as a pure standalone REST/SSE backend engine. Ask any technical question or select a prompt to begin."
    );
    m_chatLayout->insertWidget(m_chatLayout->count() - 1, banner);
}

void ChatTab::onQuickPrompt(const QString &text) {
    m_inputEdit->setPlainText(text);
    onSendClicked();
}

void ChatTab::scrollToBottom() {
    QTimer::singleShot(10, this, [this]() {
        m_scrollArea->verticalScrollBar()->setValue(m_scrollArea->verticalScrollBar()->maximum());
    });
}

void ChatTab::onSendClicked() {
    if (m_isGenerating) return;

    QString msg = m_inputEdit->toPlainText().trimmed();
    if (msg.isEmpty()) return;

    m_inputEdit->clear();

    auto *userRow = new QHBoxLayout();
    userRow->addStretch();
    auto *userCard = new MessageCard(MessageCard::User, msg);
    userCard->setMaximumWidth(700);
    userRow->addWidget(userCard);

    m_chatLayout->insertLayout(m_chatLayout->count() - 1, userRow);

    m_currentAiCard = new MessageCard(MessageCard::Assistant, "");
    m_chatLayout->insertWidget(m_chatLayout->count() - 1, m_currentAiCard);

    m_isGenerating = true;
    m_tokensReceived = 0;
    m_sendBtn->setEnabled(false);
    m_stopBtn->setEnabled(true);
    m_statusLabel->setText("⚡ Generating response from neural model via SSE stream...");

    scrollToBottom();

    m_client->sendChatStream(msg);
}

void ChatTab::onStopClicked() {
    if (!m_isGenerating) return;
    m_client->abortChatStream();
    onChatFinished();
}

void ChatTab::onClearClicked() {

    QLayoutItem *item;
    while ((item = m_chatLayout->takeAt(0)) != nullptr) {
        if (item->widget()) {
            delete item->widget();
        } else if (item->layout()) {
            QLayoutItem *subItem;
            while ((subItem = item->layout()->takeAt(0)) != nullptr) {
                if (subItem->widget()) delete subItem->widget();
                delete subItem;
            }
            delete item->layout();
        }
        delete item;
    }
    m_chatLayout->addStretch();

    addWelcomeBanner();
    m_tokenCounterLabel->setText("Tokens: 0");
    m_statusLabel->setText("Conversation cleared");
}

void ChatTab::onChunkReceived(const QString &chunk) {
    m_tokensReceived++;
    m_tokenCounterLabel->setText(QString("Tokens: %1").arg(m_tokensReceived));

    if (m_currentAiCard) {
        m_currentAiCard->appendChunk(chunk);
    }
    scrollToBottom();
}

void ChatTab::onChatFinished() {
    if (!m_isGenerating) return;
    m_isGenerating = false;
    m_sendBtn->setEnabled(true);
    m_stopBtn->setEnabled(false);
    m_statusLabel->setText(QString("Idle • Completed with %1 tokens").arg(m_tokensReceived));

    if (m_currentAiCard) {
        m_currentAiCard->finishStreaming();
        m_currentAiCard = nullptr;
    }
    scrollToBottom();
}

void ChatTab::onChatError(const QString &error) {
    m_isGenerating = false;
    m_sendBtn->setEnabled(true);
    m_stopBtn->setEnabled(false);
    m_statusLabel->setText(QString("Error: %1").arg(error));

    auto *errCard = new MessageCard(
        MessageCard::System,
        QString("<span style='color:#ef4444;'>⚠️ <b>API Communication Error:</b> %1</span>").arg(error.toHtmlEscaped())
    );
    m_chatLayout->insertWidget(m_chatLayout->count() - 1, errCard);

    if (m_currentAiCard) {
        m_currentAiCard = nullptr;
    }
    scrollToBottom();
}

