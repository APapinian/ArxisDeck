// SPDX-FileCopyrightText: 2026 Fatih Bucaklioglu
// SPDX-License-Identifier: GPL-3.0-or-later
// Minimal C ABI over Qt 6 Quick, used by src/qt.rs.
// Everything here runs on the Qt GUI thread; Rust does its per-frame work in the
// tick callback, so no cross-thread Qt calls are needed.

#include <QGuiApplication>
#include <QQmlApplicationEngine>
#include <QQuickWindow>
#include <QTimer>
#include <QImage>
#include <QVariant>
#include <QUrl>
#include <QByteArray>
#include <QString>
#include <QStringList>
#include <QMetaObject>
#include <QResource>

#include <cstring>

namespace {
int g_argc = 0;
char **g_argv = nullptr;
QGuiApplication *g_app = nullptr;
QQmlApplicationEngine *g_engine = nullptr;
QQuickWindow *g_window = nullptr;
unsigned long long g_frames = 0;

QObject *root() {
    return g_window;
}
}

extern "C" {

typedef void (*arx_tick_fn)(void *user);

int arx_init(int argc, char **argv) {
    g_argc = argc;
    g_argv = argv;
    g_app = new QGuiApplication(g_argc, g_argv);
    QGuiApplication::setApplicationName("arxis-deck");
    QGuiApplication::setApplicationDisplayName("Arxis Deck");
    QGuiApplication::setDesktopFileName("arxis-deck");
    return 0;
}

// Loads QML source. `imports` is a ':'-separated list of extra QML import folders.
// Returns 0 on success.
int arx_load(const char *qml, unsigned long len, const char *base_url, const char *imports, int fullscreen) {
    g_engine = new QQmlApplicationEngine();
    const QStringList dirs = QString::fromUtf8(imports).split(':', Qt::SkipEmptyParts);
    for (const QString &d : dirs)
        g_engine->addImportPath(d);
    if (len == 0)
        g_engine->load(QUrl(QString::fromUtf8(base_url)));   // e.g. qrc:/qml/Main.qml
    else
        g_engine->loadData(QByteArray(qml, static_cast<qsizetype>(len)), QUrl(QString::fromUtf8(base_url)));
    if (g_engine->rootObjects().isEmpty())
        return -1;
    g_window = qobject_cast<QQuickWindow *>(g_engine->rootObjects().first());
    if (!g_window)
        return -2;
    QObject::connect(g_window, &QQuickWindow::frameSwapped, [] { ++g_frames; });
    if (fullscreen) {
        g_window->showFullScreen();
    } else {
        g_window->resize(1280, 800);
        g_window->show();
    }
    return 0;
}

void arx_set_tick(arx_tick_fn cb, void *user, int interval_ms) {
    QTimer *t = new QTimer(g_app);
    QObject::connect(t, &QTimer::timeout, [cb, user] {
        cb(user);
        if (g_window)
            g_window->update();
    });
    t->start(interval_ms);
}

// Calls the QML root function `handleAction(name, source)`.
void arx_action(const char *name, const char *source) {
    if (!root())
        return;
    QMetaObject::invokeMethod(root(), "handleAction", Qt::DirectConnection,
                              Q_ARG(QVariant, QVariant(QString::fromUtf8(name))),
                              Q_ARG(QVariant, QVariant(QString::fromUtf8(source))));
}

// Calls the QML root function `method(a, b, c)` with three strings.
void arx_invoke3(const char *method, const char *a, const char *b, const char *c) {
    if (!root())
        return;
    QMetaObject::invokeMethod(root(), method, Qt::DirectConnection,
                              Q_ARG(QVariant, QVariant(QString::fromUtf8(a))),
                              Q_ARG(QVariant, QVariant(QString::fromUtf8(b))),
                              Q_ARG(QVariant, QVariant(QString::fromUtf8(c))));
}

void arx_set_property(const char *name, const char *value) {
    if (root())
        root()->setProperty(name, QVariant(QString::fromUtf8(value)));
}

// Copies the root property `name` as text into `buf`; returns the full length.
int arx_get_property(const char *name, char *buf, int cap) {
    if (!root() || cap <= 0)
        return -1;
    const QByteArray b = root()->property(name).toString().toUtf8();
    const int n = b.size() < cap - 1 ? static_cast<int>(b.size()) : cap - 1;
    std::memcpy(buf, b.constData(), n);
    buf[n] = 0;
    return static_cast<int>(b.size());
}

// Saves the window picture as PNG. Returns the number of distinct sampled colours
// (a blank picture gives 1), or -1 on failure.
int arx_grab(const char *path) {
    if (!g_window)
        return -1;
    const QImage img = g_window->grabWindow();
    if (img.isNull() || !img.save(QString::fromUtf8(path), "PNG"))
        return -1;
    QList<QRgb> seen;
    for (int y = 0; y < img.height(); y += 16)
        for (int x = 0; x < img.width(); x += 16) {
            const QRgb c = img.pixel(x, y);
            if (!seen.contains(c) && seen.size() < 256)
                seen.append(c);
        }
    return static_cast<int>(seen.size());
}

unsigned long long arx_frames() { return g_frames; }

// Registers an in-memory resource made by rcc --binary; `data` must stay valid for the run.
int arx_register_resource(const unsigned char *data) {
    return QResource::registerResource(data) ? 0 : -1;
}

// Version of the Qt library in use at run time (e.g. "6.11.2").
const char *arx_qt_version() { return qVersion(); }

int arx_window_size(int *w, int *h) {
    if (!g_window)
        return -1;
    *w = g_window->width();
    *h = g_window->height();
    return 0;
}

void arx_quit(int code) { QCoreApplication::exit(code); }

int arx_exec() {
    const int rc = g_app->exec();
    delete g_engine;
    g_engine = nullptr;
    g_window = nullptr;
    return rc;
}

} // extern "C"
