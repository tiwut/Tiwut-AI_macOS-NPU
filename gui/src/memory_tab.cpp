#include "memory_tab.h"
#include "api_client.h"
#include <QJsonArray>

MemoryTab::MemoryTab(ApiClient *client, QWidget *parent)
    : QWidget(parent)
    , m_client(client)
{
    auto *mainLayout = new QVBoxLayout(this);
    mainLayout->setContentsMargins(20, 20, 20, 20);
    mainLayout->setSpacing(14);

    auto *titleRow = new QHBoxLayout();
    auto *title = new QLabel("📚 In-RAM Hybrid Semantic Memory Bank", this);
    title->setProperty("class", "sectionHeader");
    titleRow->addWidget(title);
    titleRow->addStretch();

    auto *refreshBtn = new QPushButton("🔄 Refresh Memory", this);
    refreshBtn->setProperty("class", "secondaryBtn");
    connect(refreshBtn, &QPushButton::clicked, this, &MemoryTab::refresh);
    titleRow->addWidget(refreshBtn);
    mainLayout->addLayout(titleRow);

    auto *desc = new QLabel("Inspect all semantic vector embeddings, indexed knowledge chunks, and document sources stored in-memory inside ai.model.", this);
    desc->setProperty("class", "sectionDesc");
    mainLayout->addWidget(desc);

    auto *metricsRow = new QHBoxLayout();
    metricsRow->setSpacing(12);

    auto createCard = [this](const QString &titleText, QLabel *&valLabel) -> QFrame* {
        auto *card = new QFrame(this);
        card->setProperty("class", "glassCard");
        auto *layout = new QVBoxLayout(card);
        layout->setSpacing(4);
        auto *t = new QLabel(titleText, card);
        t->setProperty("class", "cardTitle");
        valLabel = new QLabel("-", card);
        valLabel->setProperty("class", "cardValue");
        layout->addWidget(t);
        layout->addWidget(valLabel);
        return card;
    };

    metricsRow->addWidget(createCard("Total Knowledge Chunks", m_totalChunksLabel));
    metricsRow->addWidget(createCard("Indexed Sources", m_totalDocsLabel));
    metricsRow->addWidget(createCard("Total Tokens", m_totalTokensLabel));
    metricsRow->addWidget(createCard("In-RAM Footprint", m_ramUsageLabel));
    mainLayout->addLayout(metricsRow);

    auto *bodyLayout = new QHBoxLayout();
    bodyLayout->setSpacing(14);

    auto *leftCard = new QFrame(this);
    leftCard->setProperty("class", "glassCard");
    auto *leftLayout = new QVBoxLayout(leftCard);
    leftLayout->setSpacing(8);

    auto *sourceTitle = new QLabel("📑 Ingested Data Sources", leftCard);
    sourceTitle->setProperty("class", "cardTitle");
    leftLayout->addWidget(sourceTitle);

    m_sourcesList = new QListWidget(leftCard);
    m_sourcesList->setStyleSheet(
        "QListWidget {"
        "  background-color: #0f172a;"
        "  border: 1px solid rgba(255, 255, 255, 0.1);"
        "  border-radius: 8px;"
        "  padding: 6px;"
        "  color: #f1f5f9;"
        "}"
    );
    connect(m_sourcesList, &QListWidget::itemClicked, this, &MemoryTab::onSourceSelected);
    leftLayout->addWidget(m_sourcesList, 1);

    bodyLayout->addWidget(leftCard, 1);

    auto *rightCard = new QFrame(this);
    rightCard->setProperty("class", "glassCard");
    auto *rightLayout = new QVBoxLayout(rightCard);
    rightLayout->setSpacing(8);

    auto *qaTitle = new QLabel("🔍 Instant Semantic Query & Answer Synthesis", rightCard);
    qaTitle->setProperty("class", "cardTitle");
    rightLayout->addWidget(qaTitle);

    auto *askRow = new QHBoxLayout();
    m_askInput = new QLineEdit(rightCard);
    m_askInput->setPlaceholderText("Query memory bank (e.g. 'What is Apple Silicon?', 'What is a Tensor?')");
    connect(m_askInput, &QLineEdit::returnPressed, this, &MemoryTab::onAskQuestion);
    askRow->addWidget(m_askInput, 1);

    m_askBtn = new QPushButton("Ask RAG ⚡", rightCard);
    m_askBtn->setProperty("class", "primaryBtn");
    connect(m_askBtn, &QPushButton::clicked, this, &MemoryTab::onAskQuestion);
    askRow->addWidget(m_askBtn);
    rightLayout->addLayout(askRow);

    m_askAnswerDisplay = new QTextEdit(rightCard);
    m_askAnswerDisplay->setReadOnly(true);
    m_askAnswerDisplay->setStyleSheet(
        "QTextEdit {"
        "  background-color: #0d1322;"
        "  border: 1px solid rgba(56, 189, 248, 0.2);"
        "  border-radius: 8px;"
        "  color: #f1f5f9;"
        "  padding: 10px;"
        "  font-size: 13px;"
        "}"
    );
    m_askAnswerDisplay->setHtml("<div style='color: #64748b;'>Enter a query above to test direct vector search and intelligent answer extraction from the memory bank.</div>");
    rightLayout->addWidget(m_askAnswerDisplay, 1);

    bodyLayout->addWidget(rightCard, 1);
    mainLayout->addLayout(bodyLayout, 1);

    connect(m_client, &ApiClient::memoryReceived, this, &MemoryTab::onMemoryReceived);
    connect(m_client, &ApiClient::askAnswerReceived, this, &MemoryTab::onAskAnswerReceived);
}

void MemoryTab::refresh() {
    m_client->fetchMemory();
}

void MemoryTab::onMemoryReceived(const QJsonObject &data) {
    m_lastMemoryData = data;

    int chunks = data["total_chunks"].toInt();
    int docs = data["total_documents"].toInt();
    int tokens = data["total_tokens"].toInt();
    double ramMb = data["ram_usage_mb"].toDouble();

    m_totalChunksLabel->setText(QString::number(chunks));
    m_totalDocsLabel->setText(QString::number(docs));
    m_totalTokensLabel->setText(QString::number(tokens));
    m_ramUsageLabel->setText(QString("%1 MB").arg(ramMb, 0, 'f', 2));

    m_sourcesList->clear();
    QJsonArray sources = data["sources"].toArray();
    for (const auto &s : sources) {
        m_sourcesList->addItem("📄 " + s.toString());
    }
}

void MemoryTab::onSourceSelected(QListWidgetItem *item) {
    if (!item) return;
    QString sourceName = item->text().mid(3).trimmed();
    m_askInput->setText(QString("Tell me about %1").arg(sourceName));
}

void MemoryTab::onAskQuestion() {
    QString q = m_askInput->text().trimmed();
    if (q.isEmpty()) return;

    m_askBtn->setEnabled(false);
    m_askAnswerDisplay->setHtml("<div style='color: #38bdf8;'>🧠 Searching semantic memory bank and extracting answer...</div>");
    m_client->askQuestion(q);
}

void MemoryTab::onAskAnswerReceived(const QString &question, const QString &answer) {
    m_askBtn->setEnabled(true);

    QString cleanAnswer = answer;
    QString sourceText;

    int srcIdx = cleanAnswer.indexOf("[Source:");
    if (srcIdx != -1) {
        sourceText = cleanAnswer.mid(srcIdx).trimmed();
        cleanAnswer = cleanAnswer.left(srcIdx).trimmed();
    }

    QString escapedAnswer = cleanAnswer.toHtmlEscaped();
    escapedAnswer.replace("\n\n", "<br/><br/>");
    escapedAnswer.replace("\n", "<br/>");

    QString sourceHtml;
    if (!sourceText.isEmpty()) {
        sourceHtml = QString(
            "<div style='margin-top: 10px; padding: 6px 10px; background: rgba(15,23,42,0.7); border: 1px solid rgba(56,189,248,0.3); border-radius: 6px; color: #38bdf8; font-size: 11.5px;'>"
            "  🔗 %1"
            "</div>"
        ).arg(sourceText.toHtmlEscaped());
    }

    QString html = QString(
        "<div style='margin-bottom: 8px; color: #94a3b8; font-size: 12px; font-weight: 500;'><b>Query:</b> %1</div>"
        "<div style='background-color: #1e293b; padding: 14px; border-radius: 10px; border-left: 4px solid #38bdf8; color: #f1f5f9; font-size: 13.5px; line-height: 1.5;'>"
        "  <div style='color: #f8fafc; margin-bottom: 4px;'><b>Answer:</b></div>"
        "  <div>%2</div>"
        "  %3"
        "</div>"
    ).arg(question.toHtmlEscaped(), escapedAnswer, sourceHtml);

    m_askAnswerDisplay->setHtml(html);
}

