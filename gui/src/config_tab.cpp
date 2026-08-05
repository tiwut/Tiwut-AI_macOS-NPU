#include "config_tab.h"
#include "api_client.h"
#include <QFormLayout>
#include <QMessageBox>

ConfigTab::ConfigTab(ApiClient *client, QWidget *parent)
    : QWidget(parent)
    , m_client(client)
{
    auto *mainLayout = new QVBoxLayout(this);
    mainLayout->setContentsMargins(20, 20, 20, 20);
    mainLayout->setSpacing(16);

    auto *title = new QLabel("⚙️ Hyperparameter Studio & API Configuration", this);
    title->setProperty("class", "sectionHeader");
    mainLayout->addWidget(title);

    auto *desc = new QLabel("Fine-tune neural generation temperature, sampling parameters, RAG similarity thresholds, and decoupled API server endpoints.", this);
    desc->setProperty("class", "sectionDesc");
    mainLayout->addWidget(desc);

    auto *cardsLayout = new QHBoxLayout();
    cardsLayout->setSpacing(16);

    auto *apiCard = new QFrame(this);
    apiCard->setProperty("class", "glassCard");
    auto *apiLayout = new QVBoxLayout(apiCard);
    apiLayout->setSpacing(10);

    auto *apiTitle = new QLabel("📡 Standalone API Connection", apiCard);
    apiTitle->setProperty("class", "cardTitle");
    apiLayout->addWidget(apiTitle);

    auto *apiRow = new QHBoxLayout();
    m_apiUrlInput = new QLineEdit("http://127.0.0.1:8080", apiCard);
    apiRow->addWidget(m_apiUrlInput, 1);

    m_saveApiBtn = new QPushButton("Reconnect", apiCard);
    m_saveApiBtn->setProperty("class", "secondaryBtn");
    connect(m_saveApiBtn, &QPushButton::clicked, this, &ConfigTab::onSaveApiSettings);
    apiRow->addWidget(m_saveApiBtn);
    apiLayout->addLayout(apiRow);

    auto *apiNote = new QLabel("The GUI connects to the Tiwut-AI API server asynchronously. The server can run locally or on a remote machine.", apiCard);
    apiNote->setStyleSheet("color: #64748b; font-size: 11px;");
    apiNote->setWordWrap(true);
    apiLayout->addWidget(apiNote);
    apiLayout->addStretch();

    cardsLayout->addWidget(apiCard, 1);

    auto *inferCard = new QFrame(this);
    inferCard->setProperty("class", "glassCard");
    auto *inferLayout = new QFormLayout(inferCard);
    inferLayout->setSpacing(10);

    auto *inferTitle = new QLabel("🧠 Inference & RAG Hyperparameters", inferCard);
    inferTitle->setProperty("class", "cardTitle");
    inferLayout->addRow(inferTitle);

    m_tempSpin = new QDoubleSpinBox(inferCard);
    m_tempSpin->setRange(0.05, 2.0);
    m_tempSpin->setSingleStep(0.05);
    m_tempSpin->setValue(0.6);
    inferLayout->addRow("Temperature:", m_tempSpin);

    m_topKSpin = new QSpinBox(inferCard);
    m_topKSpin->setRange(1, 100);
    m_topKSpin->setValue(40);
    inferLayout->addRow("Top-K Sampling:", m_topKSpin);

    m_topPSpin = new QDoubleSpinBox(inferCard);
    m_topPSpin->setRange(0.1, 1.0);
    m_topPSpin->setSingleStep(0.05);
    m_topPSpin->setValue(0.9);
    inferLayout->addRow("Top-P Nucleus Sampling:", m_topPSpin);

    m_repPenaltySpin = new QDoubleSpinBox(inferCard);
    m_repPenaltySpin->setRange(1.0, 2.0);
    m_repPenaltySpin->setSingleStep(0.05);
    m_repPenaltySpin->setValue(1.15);
    inferLayout->addRow("Repetition Penalty:", m_repPenaltySpin);

    m_maxTokensSpin = new QSpinBox(inferCard);
    m_maxTokensSpin->setRange(16, 2048);
    m_maxTokensSpin->setValue(250);
    inferLayout->addRow("Max Output Tokens:", m_maxTokensSpin);

    m_memThresholdSpin = new QDoubleSpinBox(inferCard);
    m_memThresholdSpin->setRange(0.05, 0.95);
    m_memThresholdSpin->setSingleStep(0.05);
    m_memThresholdSpin->setValue(0.25);
    inferLayout->addRow("Memory RAG Threshold:", m_memThresholdSpin);

    cardsLayout->addWidget(inferCard, 1);
    mainLayout->addLayout(cardsLayout, 1);

    auto *actionRow = new QHBoxLayout();
    m_statusMsgLabel = new QLabel("", this);
    m_statusMsgLabel->setStyleSheet("color: #10b981; font-weight: 500;");
    actionRow->addWidget(m_statusMsgLabel);
    actionRow->addStretch();

    m_resetModelBtn = new QPushButton("⚠️ Factory Reset Model", this);
    m_resetModelBtn->setProperty("class", "dangerBtn");
    connect(m_resetModelBtn, &QPushButton::clicked, this, &ConfigTab::onResetModel);
    actionRow->addWidget(m_resetModelBtn);

    m_saveConfigBtn = new QPushButton("💾 Save & Apply Configuration", this);
    m_saveConfigBtn->setProperty("class", "primaryBtn");
    m_saveConfigBtn->setFixedHeight(36);
    connect(m_saveConfigBtn, &QPushButton::clicked, this, &ConfigTab::onSaveConfig);
    actionRow->addWidget(m_saveConfigBtn);

    mainLayout->addLayout(actionRow);

    connect(m_client, &ApiClient::configReceived, this, &ConfigTab::onConfigReceived);
    connect(m_client, &ApiClient::configSaved, this, &ConfigTab::onConfigSaved);
    connect(m_client, &ApiClient::modelResetCompleted, this, &ConfigTab::onModelResetCompleted);
}

void ConfigTab::refresh() {
    m_client->fetchConfig();
}

void ConfigTab::onSaveApiSettings() {
    QString url = m_apiUrlInput->text().trimmed();
    m_client->setBaseUrl(url);
    m_statusMsgLabel->setText("Connecting to " + url + "...");
}

void ConfigTab::onConfigReceived(const QJsonObject &data) {
    m_currentConfig = data;
    QJsonObject infer = data["inference"].toObject();

    if (infer.contains("temperature")) m_tempSpin->setValue(infer["temperature"].toDouble());
    if (infer.contains("top_k")) m_topKSpin->setValue(infer["top_k"].toInt());
    if (infer.contains("top_p")) m_topPSpin->setValue(infer["top_p"].toDouble());
    if (infer.contains("repetition_penalty")) m_repPenaltySpin->setValue(infer["repetition_penalty"].toDouble());
    if (infer.contains("max_tokens")) m_maxTokensSpin->setValue(infer["max_tokens"].toInt());
    if (infer.contains("memory_threshold")) m_memThresholdSpin->setValue(infer["memory_threshold"].toDouble());
}

void ConfigTab::onSaveConfig() {
    QJsonObject infer = m_currentConfig["inference"].toObject();
    infer["temperature"] = m_tempSpin->value();
    infer["top_k"] = m_topKSpin->value();
    infer["top_p"] = m_topPSpin->value();
    infer["repetition_penalty"] = m_repPenaltySpin->value();
    infer["max_tokens"] = m_maxTokensSpin->value();
    infer["memory_threshold"] = m_memThresholdSpin->value();

    m_currentConfig["inference"] = infer;
    m_client->updateConfig(m_currentConfig);
    m_statusMsgLabel->setText("Saving configuration to ai.model...");
}

void ConfigTab::onConfigSaved(bool success) {
    if (success) {
        m_statusMsgLabel->setText("✅ Configuration saved & applied to ai.model");
    } else {
        m_statusMsgLabel->setText("❌ Failed to save configuration to API");
    }
}

void ConfigTab::onResetModel() {
    auto res = QMessageBox::warning(
        this,
        "Confirm Model Reset",
        "Are you sure you want to reset the neural model weights and in-RAM knowledge memory to default?",
        QMessageBox::Yes | QMessageBox::No,
        QMessageBox::No
    );

    if (res == QMessageBox::Yes) {
        m_statusMsgLabel->setText("Resetting model on API server...");
        m_client->resetModel();
    }
}

void ConfigTab::onModelResetCompleted(bool success) {
    if (success) {
        m_statusMsgLabel->setText("✅ Model & Memory reset to initial baseline");
        refresh();
    } else {
        m_statusMsgLabel->setText("❌ Model reset failed on API");
    }
}

