#pragma once

#include <QWidget>
#include <QLineEdit>
#include <QListWidget>
#include <QTextEdit>
#include <QLabel>
#include <QPushButton>
#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QJsonObject>

class ApiClient;

class MemoryTab : public QWidget {
    Q_OBJECT

public:
    explicit MemoryTab(ApiClient *client, QWidget *parent = nullptr);
    ~MemoryTab() override = default;

public slots:
    void refresh();

private slots:
    void onMemoryReceived(const QJsonObject &data);
    void onSourceSelected(QListWidgetItem *item);
    void onAskQuestion();
    void onAskAnswerReceived(const QString &question, const QString &answer);

private:
    ApiClient *m_client;

    QLabel *m_totalChunksLabel;
    QLabel *m_totalDocsLabel;
    QLabel *m_totalTokensLabel;
    QLabel *m_ramUsageLabel;

    QLineEdit *m_searchInput;
    QPushButton *m_searchBtn;
    QListWidget *m_sourcesList;
    QTextEdit *m_chunkPreview;

    QLineEdit *m_askInput;
    QPushButton *m_askBtn;
    QTextEdit *m_askAnswerDisplay;

    QJsonObject m_lastMemoryData;
};

