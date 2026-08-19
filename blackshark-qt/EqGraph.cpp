#include "EqGraph.h"

#include <QPainter>
#include <QMouseEvent>
#include <QFontMetrics>
#include <algorithm>
#include <cmath>

namespace {
// Frequency labels for the 10 bands.
const char* kFreq[EqGraph::BANDS] = {
    "31Hz", "63Hz", "125Hz", "250Hz", "500Hz",
    "1kHz", "2kHz", "4kHz", "8kHz", "16kHz"};

// Category bar segments: {label, first band index, last band index (inclusive)}.
struct Category { const char* name; int from; int to; };
const Category kCategories[] = {
    {"SUB BASS", 0, 0},
    {"BASS", 1, 2},
    {"LOW MIDS", 3, 4},
    {"MID RANGE", 5, 6},
    {"UPPER MID", 7, 7},
    {"TREBLE", 8, 9},
};

// Colors (dark theme, neon green accent).
const QColor kBg(0x1e, 0x1f, 0x22);
const QColor kGrid(0x3a, 0x3c, 0x42);
const QColor kAccent(0x43, 0xb5, 0x81);
const QColor kText(0xdb, 0xde, 0xe1);
const QColor kMuted(0x94, 0x9b, 0xa4);

constexpr int kMarginLeft = 44;   // room for dB labels
constexpr int kMarginRight = 12;
constexpr int kMarginTop = 12;
constexpr int kPlotBottom = 40;   // room for freq labels + category bar
constexpr int kPointRadius = 7;
} // namespace

EqGraph::EqGraph(QWidget* parent) : QWidget(parent) {
    setMinimumSize(560, 260);
    setMouseTracking(false);
}

QRect EqGraph::plotRect() const {
    return QRect(kMarginLeft, kMarginTop,
                 width() - kMarginLeft - kMarginRight,
                 height() - kMarginTop - kPlotBottom - 26); // 26 = category bar
}

int EqGraph::bandX(int i) const {
    QRect r = plotRect();
    if (BANDS == 1) return r.left();
    return r.left() + (r.width() * i) / (BANDS - 1);
}

int EqGraph::dbToY(double db) const {
    QRect r = plotRect();
    // db in [DB_MAX .. DB_MIN] maps to [top .. bottom]
    double frac = (DB_MAX - db) / double(DB_MAX - DB_MIN);
    return r.top() + int(std::lround(frac * r.height()));
}

double EqGraph::yToDb(int y) const {
    QRect r = plotRect();
    double frac = (y - r.top()) / double(r.height());
    frac = std::clamp(frac, 0.0, 1.0);
    double db = DB_MAX - frac * (DB_MAX - DB_MIN);
    return std::clamp(db, double(DB_MIN), double(DB_MAX));
}

int EqGraph::nearestBand(const QPoint& p) const {
    int best = -1;
    int bestDist = INT_MAX;
    for (int i = 0; i < BANDS; ++i) {
        int dx = std::abs(bandX(i) - p.x());
        if (dx < bestDist) {
            bestDist = dx;
            best = i;
        }
    }
    // Only grab a point if reasonably close horizontally.
    return (bestDist <= 30) ? best : -1;
}

void EqGraph::setBands(const std::array<int, BANDS>& bands) {
    m_bands = bands;
    for (int& b : m_bands) b = std::clamp(b, DB_MIN, DB_MAX);
    update();
}

void EqGraph::resetFlat() {
    m_bands.fill(0);
    update();
    commit();
}

void EqGraph::commit() {
    QList<int> list;
    for (int b : m_bands) list << b;
    emit eqCommitted(list);
}

void EqGraph::paintEvent(QPaintEvent*) {
    QPainter p(this);
    p.setRenderHint(QPainter::Antialiasing, true);
    p.fillRect(rect(), kBg);

    QRect r = plotRect();

    // dB grid + labels (+6, +3, 0, -3, -6)
    p.setFont(QFont(font().family(), 8));
    for (int db = DB_MAX; db >= DB_MIN; db -= 3) {
        int y = dbToY(db);
        p.setPen(QPen(kGrid, 1));
        p.drawLine(r.left(), y, r.right(), y);
        p.setPen(kMuted);
        QString lbl = (db > 0) ? QString("+%1dB").arg(db)
                    : (db == 0) ? QString("0")
                                : QString("%1dB").arg(db);
        p.drawText(QRect(0, y - 8, kMarginLeft - 6, 16),
                   Qt::AlignRight | Qt::AlignVCenter, lbl);
    }

    // Vertical band grid lines
    p.setPen(QPen(kGrid, 1));
    for (int i = 0; i < BANDS; ++i) {
        int x = bandX(i);
        p.drawLine(x, r.top(), x, r.bottom());
    }

    // EQ polyline through all points
    QPolygonF poly;
    for (int i = 0; i < BANDS; ++i) {
        poly << QPointF(bandX(i), dbToY(m_bands[i]));
    }
    p.setPen(QPen(kAccent, 2));
    p.drawPolyline(poly);

    // Control points
    p.setBrush(kAccent);
    p.setPen(Qt::NoPen);
    for (const QPointF& pt : poly) {
        p.drawEllipse(pt, kPointRadius, kPointRadius);
    }

    // Frequency labels
    p.setPen(kText);
    p.setFont(QFont(font().family(), 8));
    int freqY = r.bottom() + 4;
    for (int i = 0; i < BANDS; ++i) {
        int x = bandX(i);
        p.drawText(QRect(x - 30, freqY, 60, 14),
                   Qt::AlignHCenter | Qt::AlignVCenter, kFreq[i]);
    }

    // Category bar
    int barY = freqY + 16;
    int barH = 18;
    p.setFont(QFont(font().family(), 7));
    for (const Category& c : kCategories) {
        int x0 = bandX(c.from) - 20;
        int x1 = bandX(c.to) + 20;
        QRect seg(x0, barY, x1 - x0, barH);
        p.setPen(Qt::NoPen);
        p.setBrush(QColor(0x2b, 0x2d, 0x31));
        p.drawRect(seg);
        p.setPen(kMuted);
        p.drawText(seg, Qt::AlignCenter, c.name);
    }
}

void EqGraph::mousePressEvent(QMouseEvent* e) {
    int band = nearestBand(e->pos());
    if (band >= 0) {
        m_dragging = band;
        m_bands[band] = int(std::lround(yToDb(e->pos().y())));
        emit bandsChanged();
        update();
    }
}

void EqGraph::mouseMoveEvent(QMouseEvent* e) {
    if (m_dragging < 0) return;
    m_bands[m_dragging] = int(std::lround(yToDb(e->pos().y())));
    emit bandsChanged();
    update();
}

void EqGraph::mouseReleaseEvent(QMouseEvent*) {
    if (m_dragging < 0) return;
    m_dragging = -1;
    commit(); // send to daemon once, on release
}
