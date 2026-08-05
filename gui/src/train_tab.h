#pragma once

#include <QWidget>
#include <QLineEdit>
#include <QTextEdit>
#include <QListWidget>
#include <QPushButton>
#include <QSpinBox>
#include <QDoubleSpinBox>
#include <QCheckBox>
#include <QProgressBar>
#include <QLabel>
#include <QVBoxLayout>
#include <QHBoxLayout>

class ApiClient;

class TrainTab : public QWidget {
    Q_OBJECT

public:
    explicit TrainTab(ApiClient *client, QWidget *parent = nullptr);
    ~TrainTab() override = default;

private slots:
    void onAddUrl();
    void onAddFile();
    void onAddFolder();
    void onRemoveSelectedSource();
    void onStartTraining();
    void onTrainingFinished(bool success, const QString &msg);

private:
    ApiClient *m_client;

    QLineEdit *m_urlInput;
    QPushButton *m_addUrlBtn;
    QPushButton *m_addFileBtn;
    QPushButton *m_addFolderBtn;
    QPushButton *m_removeSourceBtn;
    QListWidget *m_sourcesList;

    QTextEdit *m_rawTextInput;
    QSpinBox *m_epochsSpin;
    QDoubleSpinBox *m_lrSpin;
    QCheckBox *m_defaultKnowledgeCheck;

    QPushButton *m_trainBtn;
    QProgressBar *m_progressBar;
    QTextEdit *m_logConsole;
    QLabel *m_statusLabel;
};

