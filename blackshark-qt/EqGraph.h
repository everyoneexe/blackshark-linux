#pragma once

#include <QWidget>
#include <array>

// Graphical 10-band equalizer widget, styled after Razer Synapse:
// a horizontal 0 dB line, 10 draggable green control points, a dB grid,
// frequency labels, and a category bar (SUB BASS .. TREBLE).
//
// Range is -6..+6 dB per band. Emits bandsChanged() live while dragging
// and eqCommitted() once when the drag ends (so the daemon is only hit
// on release, not on every pixel).
class EqGraph : public QWidget {
    Q_OBJECT
public:
    static constexpr int BANDS = 10;
    static constexpr int DB_MIN = -6;
    static constexpr int DB_MAX = 6;

    explicit EqGraph(QWidget* parent = nullptr);

    // Current band values in dB (-6..+6), index 0 = 31 Hz .. 9 = 16 kHz.
    std::array<int, BANDS> bands() const { return m_bands; }

    // Set all bands (e.g. from config or a preset). Repaints; does not emit.
    void setBands(const std::array<int, BANDS>& bands);

    // Reset all bands to 0 dB (flat) and emit eqCommitted().
    void resetFlat();

    QSize sizeHint() const override { return QSize(720, 300); }

signals:
    void bandsChanged();               // live during drag
    void eqCommitted(QList<int> bands); // once on release

protected:
    void paintEvent(QPaintEvent*) override;
    void mousePressEvent(QMouseEvent*) override;
    void mouseMoveEvent(QMouseEvent*) override;
    void mouseReleaseEvent(QMouseEvent*) override;

private:
    // Geometry helpers
    QRect plotRect() const;
    int bandX(int i) const;
    int dbToY(double db) const;
    double yToDb(int y) const;
    int nearestBand(const QPoint& p) const;

    void commit();

    std::array<int, BANDS> m_bands{}; // all zero-initialized (flat)
    int m_dragging = -1;              // index of band being dragged, or -1
};
