#include "telemetry_tab.h"
#include "api_client.h"
#include <QFormLayout>
#include <QLocale>

TelemetryTab::TelemetryTab(ApiClient *client, QWidget *parent)
    : QWidget(parent)
    , m_client(client)
{
    auto *mainLayout = new QVBoxLayout(this);
    mainLayout->setContentsMargins(20, 20, 20, 20);
    mainLayout->setSpacing(16);

    auto *headerLayout = new QHBoxLayout();
    auto *title = new QLabel("⚡ Hardware & Neural Network Telemetry Dashboard", this);
    title->setProperty("class", "sectionHeader");
    headerLayout->addWidget(title);
    headerLayout->addStretch();

    auto *refreshBtn = new QPushButton("🔄 Refresh Stats", this);
    refreshBtn->setProperty("class", "secondaryBtn");
    connect(refreshBtn, &QPushButton::clicked, this, &TelemetryTab::refresh);
    headerLayout->addWidget(refreshBtn);
    mainLayout->addLayout(headerLayout);

    auto *desc = new QLabel("Real-time telemetry regarding Apple Silicon acceleration, Transformer architecture parameters, and standalone API server health.", this);
    desc->setProperty("class", "sectionDesc");
    mainLayout->addWidget(desc);

    auto *grid = new QGridLayout();
    grid->setSpacing(14);

    auto *hwCard = new QFrame(this);
    hwCard->setProperty("class", "glassCard");
    auto *hwLayout = new QFormLayout(hwCard);
    hwLayout->setSpacing(10);

    auto *hwTitle = new QLabel("🍏 Hardware & SIMD Acceleration", hwCard);
    hwTitle->setProperty("class", "cardTitle");
    hwLayout->addRow(hwTitle);

    m_chipNameLabel = new QLabel("Apple M4 (Darwin aarch64)", hwCard);
    m_chipNameLabel->setStyleSheet("color: #38bdf8; font-weight: bold; font-size: 14px;");
    hwLayout->addRow("SoC Processor:", m_chipNameLabel);

    m_archLabel = new QLabel("aarch64 (ARM64)", hwCard);
    m_archLabel->setStyleSheet("color: #f1f5f9;");
    hwLayout->addRow("Architecture:", m_archLabel);

    m_coresLabel = new QLabel("10 Cores (Rayon Multi-threading)", hwCard);
    m_coresLabel->setStyleSheet("color: #f1f5f9;");
    hwLayout->addRow("CPU Cores:", m_coresLabel);

    m_ramLabel = new QLabel("-", hwCard);
    m_ramLabel->setStyleSheet("color: #f1f5f9;");
    hwLayout->addRow("System Memory:", m_ramLabel);

    m_simdLabel = new QLabel("ARM NEON + Apple Accelerate SIMD Active", hwCard);
    m_simdLabel->setStyleSheet("color: #10b981; font-weight: 600;");
    hwLayout->addRow("SIMD Backend:", m_simdLabel);

    grid->addWidget(hwCard, 0, 0);

    auto *modelCard = new QFrame(this);
    modelCard->setProperty("class", "glassCard");
    auto *modelLayout = new QFormLayout(modelCard);
    modelLayout->setSpacing(10);

    auto *modelTitle = new QLabel("🧠 Transformer Model Architecture", modelCard);
    modelTitle->setProperty("class", "cardTitle");
    modelLayout->addRow(modelTitle);

    m_modelParamsLabel = new QLabel("6,950,144 parameters", modelCard);
    m_modelParamsLabel->setStyleSheet("color: #38bdf8; font-weight: bold; font-size: 14px;");
    modelLayout->addRow("Parameters:", m_modelParamsLabel);

    auto *archSpecLabel = new QLabel("6 Layers • 8 Heads • 256 Embed • 1024 FFN", modelCard);
    archSpecLabel->setStyleSheet("color: #f1f5f9;");
    modelLayout->addRow("Dimensions:", archSpecLabel);

    auto *featuresLabel = new QLabel("RoPE + SwiGLU + Pre-RMSNorm", modelCard);
    featuresLabel->setStyleSheet("color: #10b981; font-weight: 600;");
    modelLayout->addRow("Innovations:", featuresLabel);

    m_vocabSizeLabel = new QLabel("Dynamic BPE (2048 tokens)", modelCard);
    m_vocabSizeLabel->setStyleSheet("color: #f1f5f9;");
    modelLayout->addRow("Tokenizer:", m_vocabSizeLabel);

    m_modelPathLabel = new QLabel("ai.model (Unified Container)", modelCard);
    m_modelPathLabel->setStyleSheet("color: #f1f5f9;");
    modelLayout->addRow("Package:", m_modelPathLabel);

    grid->addWidget(modelCard, 0, 1);

    auto *serverCard = new QFrame(this);
    serverCard->setProperty("class", "glassCard");
    auto *serverLayout = new QFormLayout(serverCard);
    serverLayout->setSpacing(10);

    auto *serverTitle = new QLabel("📡 Decoupled API Server Status", serverCard);
    serverTitle->setProperty("class", "cardTitle");
    serverLayout->addRow(serverTitle);

    m_serverStatusLabel = new QLabel("Connected (HTTP/1.1 & SSE)", serverCard);
    m_serverStatusLabel->setStyleSheet("color: #10b981; font-weight: bold;");
    serverLayout->addRow("API State:", m_serverStatusLabel);

    m_serverUrlLabel = new QLabel("http://127.0.0.1:8080", serverCard);
    m_serverUrlLabel->setStyleSheet("color: #38bdf8;");
    serverLayout->addRow("Endpoint:", m_serverUrlLabel);

    m_engineLabel = new QLabel("Tiwut-AI v2 (Native Pure Rust Engine)", serverCard);
    m_engineLabel->setStyleSheet("color: #f1f5f9;");
    serverLayout->addRow("Backend Engine:", m_engineLabel);

    grid->addWidget(serverCard, 1, 0, 1, 2);

    mainLayout->addLayout(grid, 1);

    connect(m_client, &ApiClient::statusReceived, this, &TelemetryTab::onStatusReceived);
}

void TelemetryTab::refresh() {
    m_client->fetchStatus();
}

void TelemetryTab::onStatusReceived(const QJsonObject &data) {
    QJsonObject hw = data["hardware"].toObject();

    QString chip = hw["chip_name"].toString();
    if (!chip.isEmpty()) m_chipNameLabel->setText(chip);

    QString arch = hw["architecture"].toString();
    if (!arch.isEmpty()) m_archLabel->setText(arch);

    int cores = hw["cpu_cores"].toInt();
    if (cores > 0) m_coresLabel->setText(QString("%1 Cores (Rayon Multi-threading)").arg(cores));

    double totalRam = hw["total_memory_mb"].toDouble() / 1024.0;
    double usedRam = hw["used_memory_mb"].toDouble() / 1024.0;
    m_ramLabel->setText(QString("%1 GB / %2 GB").arg(usedRam, 0, 'f', 1).arg(totalRam, 0, 'f', 1));

    int totalParams = data["total_parameters"].toInt();
    if (totalParams > 0) {
        QLocale locale;
        m_modelParamsLabel->setText(QString("%1 parameters").arg(locale.toString(totalParams)));
    }

    int vocab = data["vocab_size"].toInt();
    if (vocab > 0) {
        m_vocabSizeLabel->setText(QString("Dynamic BPE (%1 tokens)").arg(vocab));
    }

    QString modelPath = data["model_path"].toString();
    if (!modelPath.isEmpty()) {
        m_modelPathLabel->setText(modelPath);
    }

    QString engine = data["engine"].toString();
    if (!engine.isEmpty()) {
        m_engineLabel->setText(engine);
    }
}

