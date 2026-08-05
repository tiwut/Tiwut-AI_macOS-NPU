#include "train_tab.h"
#include "api_client.h"
#include <QFileDialog>
#include <QGroupBox>
#include <QFormLayout>
#include <QDateTime>

TrainTab::TrainTab(ApiClient *client, QWidget *parent)
    : QWidget(parent)
    , m_client(client)
{
    auto *mainLayout = new QVBoxLayout(this);
    mainLayout->setContentsMargins(20, 20, 20, 20);
    mainLayout->setSpacing(14);

    auto *title = new QLabel("🧠 Dynamic Neural Training & Knowledge Ingestion", this);
    title->setProperty("class", "sectionHeader");
    mainLayout->addWidget(title);

    auto *desc = new QLabel("Train the pure Transformer core on web URLs, local documents, code repositories, or custom text. All knowledge and weights are bundled directly into ai.model.", this);
    desc->setProperty("class", "sectionDesc");
    mainLayout->addWidget(desc);

    auto *bodyLayout = new QHBoxLayout();
    bodyLayout->setSpacing(16);

    auto *leftCol = new QVBoxLayout();
    leftCol->setSpacing(12);

    auto *sourceCard = new QFrame(this);
    sourceCard->setProperty("class", "glassCard");
    auto *sourceCardLayout = new QVBoxLayout(sourceCard);
    sourceCardLayout->setSpacing(8);

    auto *sourceLabel = new QLabel("📥 Training Data Sources", sourceCard);
    sourceLabel->setProperty("class", "cardTitle");
    sourceCardLayout->addWidget(sourceLabel);

    auto *urlRow = new QHBoxLayout();
    m_urlInput = new QLineEdit(sourceCard);
    m_urlInput->setPlaceholderText("https://en.wikipedia.org/wiki/Artificial_intelligence");
    urlRow->addWidget(m_urlInput, 1);

    m_addUrlBtn = new QPushButton("Add URL", sourceCard);
    m_addUrlBtn->setProperty("class", "secondaryBtn");
    connect(m_addUrlBtn, &QPushButton::clicked, this, &TrainTab::onAddUrl);
    urlRow->addWidget(m_addUrlBtn);
    sourceCardLayout->addLayout(urlRow);

    auto *fileRow = new QHBoxLayout();
    m_addFileBtn = new QPushButton("📁 Add File(s)", sourceCard);
    m_addFileBtn->setProperty("class", "secondaryBtn");
    connect(m_addFileBtn, &QPushButton::clicked, this, &TrainTab::onAddFile);
    fileRow->addWidget(m_addFileBtn);

    m_addFolderBtn = new QPushButton("📂 Add Folder", sourceCard);
    m_addFolderBtn->setProperty("class", "secondaryBtn");
    connect(m_addFolderBtn, &QPushButton::clicked, this, &TrainTab::onAddFolder);
    fileRow->addWidget(m_addFolderBtn);

    m_removeSourceBtn = new QPushButton("🗑️ Remove", sourceCard);
    m_removeSourceBtn->setProperty("class", "dangerBtn");
    connect(m_removeSourceBtn, &QPushButton::clicked, this, &TrainTab::onRemoveSelectedSource);
    fileRow->addWidget(m_removeSourceBtn);
    sourceCardLayout->addLayout(fileRow);

    m_sourcesList = new QListWidget(sourceCard);
    m_sourcesList->setStyleSheet(
        "QListWidget {"
        "  background-color: #0f172a;"
        "  border: 1px solid rgba(255, 255, 255, 0.1);"
        "  border-radius: 6px;"
        "  color: #f1f5f9;"
        "  padding: 4px;"
        "  min-height: 80px;"
        "  max-height: 120px;"
        "}"
    );
    sourceCardLayout->addWidget(m_sourcesList);

    leftCol->addWidget(sourceCard);

    auto *paramCard = new QFrame(this);
    paramCard->setProperty("class", "glassCard");
    auto *paramLayout = new QFormLayout(paramCard);
    paramLayout->setSpacing(10);

    auto *paramLabel = new QLabel("⚙️ Training Hyperparameters", paramCard);
    paramLabel->setProperty("class", "cardTitle");
    paramLayout->addRow(paramLabel);

    m_epochsSpin = new QSpinBox(paramCard);
    m_epochsSpin->setRange(1, 100);
    m_epochsSpin->setValue(8);
    m_epochsSpin->setStyleSheet("background-color: #0f172a; color: #f1f5f9; padding: 4px; border-radius: 4px;");
    paramLayout->addRow("Training Epochs:", m_epochsSpin);

    m_lrSpin = new QDoubleSpinBox(paramCard);
    m_lrSpin->setRange(0.00001, 0.01);
    m_lrSpin->setSingleStep(0.0001);
    m_lrSpin->setDecimals(5);
    m_lrSpin->setValue(0.0004);
    m_lrSpin->setStyleSheet("background-color: #0f172a; color: #f1f5f9; padding: 4px; border-radius: 4px;");
    paramLayout->addRow("Learning Rate (AdamW):", m_lrSpin);

    m_defaultKnowledgeCheck = new QCheckBox("Bootstrap Built-in English Knowledge Base", paramCard);
    m_defaultKnowledgeCheck->setChecked(false);
    m_defaultKnowledgeCheck->setStyleSheet("color: #e2e8f0; font-weight: 500;");
    paramLayout->addRow("", m_defaultKnowledgeCheck);

    leftCol->addWidget(paramCard);

    auto *textCard = new QFrame(this);
    textCard->setProperty("class", "glassCard");
    auto *textLayout = new QVBoxLayout(textCard);
    auto *textLabel = new QLabel("📝 Paste Custom Text Directly (Optional)", textCard);
    textLabel->setProperty("class", "cardTitle");
    textLayout->addWidget(textLabel);

    m_rawTextInput = new QTextEdit(textCard);
    m_rawTextInput->setPlaceholderText("Paste raw markdown, research notes, or technical definitions here...");
    m_rawTextInput->setMaximumHeight(90);
    textLayout->addWidget(m_rawTextInput);
    leftCol->addWidget(textCard);

    bodyLayout->addLayout(leftCol, 1);

    auto *rightCol = new QVBoxLayout();
    rightCol->setSpacing(12);

    auto *logCard = new QFrame(this);
    logCard->setProperty("class", "glassCard");
    auto *logLayout = new QVBoxLayout(logCard);
    logLayout->setSpacing(10);

    auto *logTitle = new QLabel("📊 Real-time Training Console & Loss Curve", logCard);
    logTitle->setProperty("class", "cardTitle");
    logLayout->addWidget(logTitle);

    m_progressBar = new QProgressBar(logCard);
    m_progressBar->setRange(0, 100);
    m_progressBar->setValue(0);
    logLayout->addWidget(m_progressBar);

    m_logConsole = new QTextEdit(logCard);
    m_logConsole->setReadOnly(true);
    m_logConsole->setStyleSheet(
        "QTextEdit {"
        "  background-color: #050811;"
        "  border: 1px solid rgba(56, 189, 248, 0.2);"
        "  border-radius: 8px;"
        "  font-family: Menlo, Monaco, 'Courier New', monospace;"
        "  font-size: 11.5px;"
        "  color: #38bdf8;"
        "  padding: 10px;"
        "}"
    );
    m_logConsole->append("⚡ Training Studio Ready.\nConfigure sources and click 'Start Neural Training' below.");
    logLayout->addWidget(m_logConsole, 1);

    rightCol->addWidget(logCard, 1);
    bodyLayout->addLayout(rightCol, 1);
    mainLayout->addLayout(bodyLayout, 1);

    auto *actionRow = new QHBoxLayout();
    m_statusLabel = new QLabel("Status: Idle", this);
    m_statusLabel->setStyleSheet("color: #94a3b8; font-size: 12px; font-weight: 500;");
    actionRow->addWidget(m_statusLabel);
    actionRow->addStretch();

    m_trainBtn = new QPushButton("🚀 Start Neural Training Studio", this);
    m_trainBtn->setProperty("class", "primaryBtn");
    m_trainBtn->setFixedHeight(38);
    connect(m_trainBtn, &QPushButton::clicked, this, &TrainTab::onStartTraining);
    actionRow->addWidget(m_trainBtn);

    mainLayout->addLayout(actionRow);

    connect(m_client, &ApiClient::trainingFinished, this, &TrainTab::onTrainingFinished);
}

void TrainTab::onAddUrl() {
    QString url = m_urlInput->text().trimmed();
    if (!url.isEmpty()) {
        m_sourcesList->addItem("[URL] " + url);
        m_urlInput->clear();
    }
}

void TrainTab::onAddFile() {
    QStringList files = QFileDialog::getOpenFileNames(this, "Select Training Text / Markdown Files", "", "Documents (*.txt *.md *.rs *.py *.cpp *.h *.json *.csv);;All Files (*)");
    for (const auto &f : files) {
        m_sourcesList->addItem("[FILE] " + f);
    }
}

void TrainTab::onAddFolder() {
    QString dir = QFileDialog::getExistingDirectory(this, "Select Training Directory");
    if (!dir.isEmpty()) {
        m_sourcesList->addItem("[DIR] " + dir);
    }
}

void TrainTab::onRemoveSelectedSource() {
    qDeleteAll(m_sourcesList->selectedItems());
}

void TrainTab::onStartTraining() {
    QStringList urls;
    QStringList files;

    for (int i = 0; i < m_sourcesList->count(); ++i) {
        QString text = m_sourcesList->item(i)->text();
        if (text.startsWith("[URL] ")) {
            urls.append(text.mid(6).trimmed());
        } else if (text.startsWith("[FILE] ") || text.startsWith("[DIR] ")) {
            files.append(text.mid(7).trimmed());
        }
    }

    QString rawText = m_rawTextInput->toPlainText().trimmed();
    int epochs = m_epochsSpin->value();
    double lr = m_lrSpin->value();
    bool incDef = m_defaultKnowledgeCheck->isChecked();

    if (urls.isEmpty() && files.isEmpty() && rawText.isEmpty() && !incDef) {
        m_logConsole->append("\n⚠️ Please specify at least one URL, file, folder, custom text, or enable built-in knowledge.");
        return;
    }

    m_trainBtn->setEnabled(false);
    m_progressBar->setValue(20);
    m_statusLabel->setText("Status: Training in progress on API backend...");

    QString timestamp = QDateTime::currentDateTime().toString("HH:mm:ss");
    m_logConsole->append(QString("\n[%1] 🚀 Dispatched training job to Tiwut-AI API backend...").arg(timestamp));
    m_logConsole->append(QString("  • Epochs: %1 | LR: %2 | Include Default: %3").arg(epochs).arg(lr).arg(incDef ? "true" : "false"));

    m_client->startTraining(urls, files, rawText, epochs, lr, incDef);
}

void TrainTab::onTrainingFinished(bool success, const QString &msg) {
    m_trainBtn->setEnabled(true);
    m_progressBar->setValue(success ? 100 : 0);
    m_statusLabel->setText(success ? "Status: Training Completed Successfully" : "Status: Training Failed");

    QString timestamp = QDateTime::currentDateTime().toString("HH:mm:ss");
    if (success) {
        m_logConsole->append(QString("\n[%1] ✅ %2").arg(timestamp).arg(msg));
        m_logConsole->append("💾 All neural weights and RAG memory chunks updated in 'ai.model'.");
    } else {
        m_logConsole->append(QString("\n[%1] ❌ %2").arg(timestamp).arg(msg));
    }
}

