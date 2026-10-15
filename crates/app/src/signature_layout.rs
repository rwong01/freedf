//! Signature geometry stays in PDF points, independent of zoom, DPI and scrolling.
use openpdf_document::{Point, Rect};

pub fn center(r: Rect) -> Point {
    Point {
        x: (r.left + r.right) / 2.,
        y: (r.bottom + r.top) / 2.,
    }
}

pub fn fit_in(source: Rect, target: Rect) -> Rect {
    let scale = (target.width() / source.width().max(0.01))
        .min(target.height() / source.height().max(0.01));
    let (w, h) = (source.width() * scale, source.height() * scale);
    let c = center(target);
    Rect {
        left: c.x - w / 2.,
        right: c.x + w / 2.,
        bottom: c.y - h / 2.,
        top: c.y + h / 2.,
    }
}

pub fn within(source: Rect, crop: Rect) -> Rect {
    let scale = (crop.width() / source.width().max(0.01))
        .min(crop.height() / source.height().max(0.01))
        .min(1.);
    let w = source.width() * scale;
    let h = source.height() * scale;
    let left = source
        .left
        .clamp(crop.left, (crop.right - w).max(crop.left));
    let bottom = source
        .bottom
        .clamp(crop.bottom, (crop.top - h).max(crop.bottom));
    Rect {
        left,
        bottom,
        right: left + w,
        top: bottom + h,
    }
}

pub fn auto_size(source: Rect, position: Point, crop: Rect) -> Rect {
    let w = (crop.width() * 0.25).min(180.);
    let h = (crop.height() * 0.09).min(54.);
    within(
        fit_in(
            source,
            Rect {
                left: position.x - w / 2.,
                right: position.x + w / 2.,
                bottom: position.y - h / 2.,
                top: position.y + h / 2.,
            },
        ),
        crop,
    )
}

pub fn scaled(source: Rect, factor: f32, crop: Rect) -> Rect {
    let c = center(source);
    let factor = factor.max(12. / source.width().max(0.01));
    within(
        Rect {
            left: c.x - source.width() * factor / 2.,
            right: c.x + source.width() * factor / 2.,
            bottom: c.y - source.height() * factor / 2.,
            top: c.y + source.height() * factor / 2.,
        },
        crop,
    )
}

/// Resize about the opposite corner without flipping or stretching the artwork.
pub fn resize(source: Rect, anchor: Point, pointer: Point, crop: Rect) -> Rect {
    let c = center(source);
    let sx = if c.x >= anchor.x { 1. } else { -1. };
    let sy = if c.y >= anchor.y { 1. } else { -1. };
    let w = source.width().max(0.01);
    let h = source.height().max(0.01);
    // Project the pointer onto the aspect-ratio diagonal; either axis can resize.
    let desired = (((pointer.x - anchor.x) * sx * w + (pointer.y - anchor.y) * sy * h)
        / (w * w + h * h))
        .max(12. / w);
    let max_w = if sx > 0. {
        crop.right - anchor.x
    } else {
        anchor.x - crop.left
    };
    let max_h = if sy > 0. {
        crop.top - anchor.y
    } else {
        anchor.y - crop.bottom
    };
    let scale = desired.min(max_w / w).min(max_h / h).max(0.001);
    Rect::new(
        anchor,
        Point {
            x: anchor.x + sx * w * scale,
            y: anchor.y + sy * h * scale,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use openpdf_document::PageGeometry;
    #[test]
    fn placement_fits_small_pages_and_preserves_proportions() {
        let source = Rect {
            left: 0.,
            bottom: 0.,
            right: 600.,
            top: 100.,
        };
        let crop = Rect {
            left: 20.,
            bottom: 40.,
            right: 320.,
            top: 440.,
        };
        for p in [
            Point { x: 20., y: 40. },
            Point { x: 320., y: 440. },
            center(crop),
        ] {
            let r = auto_size(source, p, crop);
            assert!(
                r.left >= crop.left
                    && r.right <= crop.right
                    && r.bottom >= crop.bottom
                    && r.top <= crop.top
            );
            assert!((r.width() / r.height() - 6.).abs() < 0.001);
            assert!(r.width() <= 75.01);
        }
        let target = Rect {
            left: 70.,
            bottom: 100.,
            right: 130.,
            top: 120.,
        };
        assert_eq!(
            fit_in(source, target),
            Rect {
                left: 70.,
                bottom: 105.,
                right: 130.,
                top: 115.
            }
        );
    }
    #[test]
    fn corner_resize_is_reversible_across_view_rotations() {
        let crop = Rect {
            left: 20.,
            bottom: 30.,
            right: 620.,
            top: 830.,
        };
        let original = Rect {
            left: 200.,
            bottom: 200.,
            right: 380.,
            top: 260.,
        };
        let g = PageGeometry { crop, rotation: 90 };
        for rotation in [0, 90, 180, 270] {
            for anchor in [Point { x: 200., y: 200. }, Point { x: 380., y: 260. }] {
                let c = center(original);
                let desired = Point {
                    x: anchor.x + 2. * (c.x - anchor.x) * 1.5,
                    y: anchor.y + 2. * (c.y - anchor.y) * 1.5,
                };
                let screen = g.to_display(desired, rotation);
                let r = resize(original, anchor, g.from_display(screen, rotation), crop);
                assert!((r.width() - 270.).abs() < 0.001);
                assert!((r.width() / r.height() - 3.).abs() < 0.001);
                assert!(r.contains(anchor));
            }
        }
    }

    #[test]
    fn resized_ink_signature_survives_undo_save_and_reopen() {
        use openpdf_document::{Annotation, AnnotationKind, DocumentModel, fixtures};
        let mut doc = DocumentModel::open(fixtures::sample(1, false), None).unwrap();
        let source = Rect {
            left: 0.,
            bottom: 0.,
            right: 180.,
            top: 60.,
        };
        let mut a = Annotation::new(900, 0, AnnotationKind::Ink, source);
        a.signature = true;
        a.strokes = vec![vec![
            Point { x: 5., y: 5. },
            Point { x: 70., y: 55. },
            Point { x: 175., y: 8. },
        ]];
        a.move_to(auto_size(
            a.rect,
            Point { x: 300., y: 300. },
            doc.pages[0].crop,
        ));
        doc.change(|e| e.annotations.push(a.clone())).unwrap();
        let original = a.clone();
        a.move_to(scaled(a.rect, 1.5, doc.pages[0].crop));
        doc.change(|e| e.annotations[0] = a.clone()).unwrap();
        doc.undo();
        assert_eq!(doc.edits.annotations[0], original);
        doc.redo();
        assert_eq!(doc.edits.annotations[0], a);
        let reopened = DocumentModel::open(doc.serialize().unwrap(), None).unwrap();
        assert_eq!(reopened.edits.annotations[0], a);
        let mut legacy = serde_json::to_value(&a).unwrap();
        legacy.as_object_mut().unwrap().remove("signature");
        assert!(
            !serde_json::from_value::<Annotation>(legacy)
                .unwrap()
                .signature
        );
    }
}
