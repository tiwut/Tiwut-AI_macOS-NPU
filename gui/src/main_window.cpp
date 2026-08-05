#include "main_window.h"
#include "api_client.h"
#include "chat_tab.h"
#include "train_tab.h"
#include "memory_tab.h"
#include "telemetry_tab.h"
#include "config_tab.h"

#include <QHBoxLayout>
#include <QVBoxLayout>
#include <QFrame>

MainWindow::MainWindow(QWidget *parent)
    : QMainWindow(parent)
    , m_client(new ApiClient(this))
{
    setupUi();
    connect(m_client, &ApiClient::healthStatusChanged, this, &MainWindow::onHealthStatusChanged);
    m_client->checkHealth();
}

void MainWindow::setupUi() {
    setWindowTitle("Tiwut-AI v2 — High-Performance Neural Engine GUI");
    resize(1150, 750);
    setMinimumSize(950, 620);

    auto *centralWidget = new QWidget(this);
    centralWidget->setObjectName("centralWidget");
    setCentralWidget(centralWidget);

    auto *rootLayout = new QHBoxLayout(centralWidget);
    rootLayout->setContentsMargins(0, 0, 0, 0);
    rootLayout->setSpacing(0);

    auto *sidebar = new QFrame(this);
    sidebar->setObjectName("sidebar");
    auto *sideLayout = new QVBoxLayout(sidebar);
    sideLayout->setContentsMargins(12, 20, 12, 20);
    sideLayout->setSpacing(8);

    auto *logoTitle = new QLabel("⚡ Tiwut-AI v2", sidebar);
    logoTitle->setObjectName("logoTitle");
    sideLayout->addWidget(logoTitle);

    auto *logoSubtitle = new QLabel("Rust AI Engine • Apple Silicon", sidebar);
    logoSubtitle->setObjectName("logoSubtitle");
    sideLayout->addWidget(logoSubtitle);

    m_navGroup = new QButtonGroup(this);
    m_navGroup->setExclusive(true);

    auto createNavButton = [this, sidebar, sideLayout](int id, const QString &text) -> QPushButton* {
        auto *btn = new QPushButton(text, sidebar);
        btn->setProperty("class", "navBtn");
        btn->setCheckable(true);
        btn->setCursor(Qt::PointingHandCursor);
        m_navGroup->addButton(btn, id);
        sideLayout->addWidget(btn);
        return btn;
    };

    m_chatNavBtn = createNavButton(0, "💬  Neural Chat");
    m_trainNavBtn = createNavButton(1, "🧠  Training Studio");
    m_memoryNavBtn = createNavButton(2, "📚  Memory Bank");
    m_telemetryNavBtn = createNavButton(3, "⚡  Hardware Stats");
    m_configNavBtn = createNavButton(4, "⚙️  Configuration");

    m_chatNavBtn->setChecked(true);

    connect(m_navGroup, &QButtonGroup::idClicked, this, &MainWindow::onNavButtonClicked);

    sideLayout->addStretch();

    auto *statusFrame = new QFrame(sidebar);
    statusFrame->setStyleSheet("background: rgba(15, 23, 42, 0.6); border: 1px solid rgba(255,255,255,0.06); border-radius: 8px; padding: 6px;");
    auto *statusLayout = new QHBoxLayout(statusFrame);
    statusLayout->setContentsMargins(6, 6, 6, 6);
    statusLayout->setSpacing(8);

    m_statusDot = new QLabel(statusFrame);
    m_statusDot->setObjectName("statusDotDisconnected");
    statusLayout->addWidget(m_statusDot);

    m_statusText = new QLabel("Connecting API...", statusFrame);
    m_statusText->setStyleSheet("color: #94a3b8; font-size: 11px; font-weight: 500;");
    statusLayout->addWidget(m_statusText, 1);

    sideLayout->addWidget(statusFrame);
    rootLayout->addWidget(sidebar);

    m_stackedWidget = new QStackedWidget(this);

    m_chatTab = new ChatTab(m_client, this);
    m_trainTab = new TrainTab(m_client, this);
    m_memoryTab = new MemoryTab(m_client, this);
    m_telemetryTab = new TelemetryTab(m_client, this);
    m_configTab = new ConfigTab(m_client, this);

    m_stackedWidget->addWidget(m_chatTab);
    m_stackedWidget->addWidget(m_trainTab);
    m_stackedWidget->addWidget(m_memoryTab);
    m_stackedWidget->addWidget(m_telemetryTab);
    m_stackedWidget->addWidget(m_configTab);

    rootLayout->addWidget(m_stackedWidget, 1);
}

void MainWindow::onNavButtonClicked(int id) {
    m_stackedWidget->setCurrentIndex(id);
    if (id == 2) m_memoryTab->refresh();
    if (id == 3) m_telemetryTab->refresh();
    if (id == 4) m_configTab->refresh();
}

void MainWindow::onHealthStatusChanged(bool connected, const QString &info) {
    if (connected) {
        m_statusDot->setObjectName("statusDotConnected");
        m_statusText->setText("API Online (8080)");
        m_statusText->setStyleSheet("color: #10b981; font-size: 11px; font-weight: 500;");
    } else {
        m_statusDot->setObjectName("statusDotDisconnected");
        m_statusText->setText("API Offline");
        m_statusText->setStyleSheet("color: #ef4444; font-size: 11px; font-weight: 500;");
    }
    m_statusDot->style()->unpolish(m_statusDot);
    m_statusDot->style()->polish(m_statusDot);
}

