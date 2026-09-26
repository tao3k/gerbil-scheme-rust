//! Collect source-line marker dependencies before generating cached scanners.

use std::collections::BTreeSet;

use super::{
    EventComputedOffsetIr, EventOffsetIr, EventPredicateIr, EventStatementIr, EventUsizeIr,
};

pub(super) fn collect_line_markers(
    statements: &[EventStatementIr],
    markers: &mut BTreeSet<(u8, u8)>,
) {
    for statement in statements {
        match statement {
            EventStatementIr::SetUsize { value, .. }
            | EventStatementIr::CloseThroughLevel { level: value, .. }
            | EventStatementIr::OpenLevel { level: value, .. }
            | EventStatementIr::PushFrame { value, .. } => {
                collect_usize_markers(value, markers);
            }
            EventStatementIr::SetBool { value, .. } => collect_predicate_markers(value, markers),
            EventStatementIr::Token { start, end, .. } => {
                collect_offset_markers(start, markers);
                collect_offset_markers(end, markers);
            }
            EventStatementIr::If {
                condition,
                consequent,
                alternate,
            } => {
                collect_predicate_markers(condition, markers);
                collect_line_markers(consequent, markers);
                collect_line_markers(alternate, markers);
            }
            EventStatementIr::ForLineBytes {
                from, until, body, ..
            } => {
                collect_offset_markers(from, markers);
                collect_offset_markers(until, markers);
                collect_line_markers(body, markers);
            }
            EventStatementIr::WithSourceBounds { from, until, .. }
            | EventStatementIr::CallSourceHelper { from, until, .. } => {
                collect_offset_markers(from, markers);
                collect_offset_markers(until, markers);
            }
            EventStatementIr::CloseFramesWhile { condition, .. } => {
                collect_predicate_markers(condition, markers);
            }
            _ => {}
        }
    }
}

fn collect_offset_markers(offset: &EventOffsetIr, markers: &mut BTreeSet<(u8, u8)>) {
    match offset {
        EventOffsetIr::Computed(EventComputedOffsetIr::LineMarkerEnd { marker, separator }) => {
            markers.insert((*marker, *separator));
        }
        EventOffsetIr::Computed(
            EventComputedOffsetIr::LineSkipHorizontal { from }
            | EventComputedOffsetIr::LineScanWord { from }
            | EventComputedOffsetIr::LineScanKey { from }
            | EventComputedOffsetIr::LineScanNonspaceUntil { from, .. }
            | EventComputedOffsetIr::LineScanUntil { from, .. }
            | EventComputedOffsetIr::LinePhysicalEnd { from }
            | EventComputedOffsetIr::LineStep { from }
            | EventComputedOffsetIr::LineTrimEndFrom { from },
        ) => collect_offset_markers(from, markers),
        _ => {}
    }
}

fn collect_predicate_markers(predicate: &EventPredicateIr, markers: &mut BTreeSet<(u8, u8)>) {
    match predicate {
        EventPredicateIr::UsizePositive { value } => collect_usize_markers(value, markers),
        EventPredicateIr::UsizeEqual { left, right }
        | EventPredicateIr::UsizeNotEqual { left, right }
        | EventPredicateIr::UsizeGreater { left, right } => {
            collect_usize_markers(left, markers);
            collect_usize_markers(right, markers);
        }
        EventPredicateIr::OffsetLess { left, right } => {
            collect_offset_markers(left, markers);
            collect_offset_markers(right, markers);
        }
        EventPredicateIr::LineByteEqual { at, .. } => collect_offset_markers(at, markers),
        EventPredicateIr::LineBytesAllIn { from, until, .. }
        | EventPredicateIr::LineBytesAnyIn { from, until, .. }
        | EventPredicateIr::LineBytesInSet { from, until, .. } => {
            collect_offset_markers(from, markers);
            collect_offset_markers(until, markers);
        }
        EventPredicateIr::SourceSlicesEqual {
            left_from,
            left_until,
            right_from,
            right_until,
            ..
        } => {
            for offset in [left_from, left_until, right_from, right_until] {
                collect_offset_markers(offset, markers);
            }
        }
        EventPredicateIr::Not { value } => collect_predicate_markers(value, markers),
        EventPredicateIr::And { left, right } | EventPredicateIr::Or { left, right } => {
            collect_predicate_markers(left, markers);
            collect_predicate_markers(right, markers);
        }
        _ => {}
    }
}

fn collect_usize_markers(value: &EventUsizeIr, markers: &mut BTreeSet<(u8, u8)>) {
    match value {
        EventUsizeIr::LineMarkerLevel { marker, separator } => {
            markers.insert((*marker, *separator));
        }
        EventUsizeIr::Offset { value } => collect_offset_markers(value, markers),
        EventUsizeIr::Add { left, right }
        | EventUsizeIr::Multiply { left, right }
        | EventUsizeIr::Divide { left, right } => {
            collect_usize_markers(left, markers);
            collect_usize_markers(right, markers);
        }
        _ => {}
    }
}
