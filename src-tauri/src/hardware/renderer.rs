use std::{fs, path::PathBuf};

use ab_glyph::{point, Font, FontArc, PxScale, ScaleFont};

use crate::output::{expiration_at, PreparationContainerLabelData, PreparationOutput};

use super::{HardwareError, LabelPrinterConfig, PrinterLanguage};

pub(crate) const LABEL_RENDERER_VERSION: &str = "oncoflow-raw-label-raster-v7";
const MAX_PIXELS: usize = 25_000_000;

pub(crate) fn render_preparation_label(
    output: &PreparationOutput,
    config: &LabelPrinterConfig,
) -> Result<Vec<u8>, HardwareError> {
    let dimensions = validate_dimensions(config)?;
    let font = load_thai_capable_font()?;
    let mut payload = Vec::new();
    for container in &output.containers {
        let bitmap =
            render_label_bitmap(output, container, &font, dimensions.0, dimensions.1, config)?;
        payload.extend(match config.language {
            PrinterLanguage::Escpos => encode_escpos(&bitmap),
            PrinterLanguage::Tspl => encode_tspl(&bitmap, config),
        });
    }
    Ok(payload)
}

pub(crate) fn render_preparation_labels(
    outputs: &[PreparationOutput],
    config: &LabelPrinterConfig,
) -> Result<Vec<u8>, HardwareError> {
    if outputs.is_empty() {
        return Err(HardwareError::InvalidConfig("preparationIds"));
    }
    let dimensions = validate_dimensions(config)?;
    let font = load_thai_capable_font()?;
    let mut payload = Vec::new();
    for output in outputs {
        for container in &output.containers {
            let bitmap =
                render_label_bitmap(output, container, &font, dimensions.0, dimensions.1, config)?;
            payload.extend(match config.language {
                PrinterLanguage::Escpos => encode_escpos(&bitmap),
                PrinterLanguage::Tspl => encode_tspl(&bitmap, config),
            });
        }
    }
    Ok(payload)
}

pub(super) fn render_test_label(config: &LabelPrinterConfig) -> Result<Vec<u8>, HardwareError> {
    let dimensions = validate_dimensions(config)?;
    let font = load_thai_capable_font()?;
    let mut bitmap = MonochromeBitmap::new(dimensions.0, dimensions.1);
    draw_text(&mut bitmap, &font, "OncoFlow printer test", 12, 16, 26.0, 2);
    draw_text(&mut bitmap, &font, "ทดสอบเครื่องพิมพ์ฉลาก", 12, 58, 22.0, 2);
    draw_text(&mut bitmap, &font, LABEL_RENDERER_VERSION, 12, 96, 14.0, 2);
    match config.language {
        PrinterLanguage::Escpos => Ok(encode_escpos(&bitmap)),
        PrinterLanguage::Tspl => Ok(encode_tspl(&bitmap, config)),
    }
}

fn validate_dimensions(config: &LabelPrinterConfig) -> Result<(u32, u32), HardwareError> {
    if config.spooler_name.trim().is_empty() || config.spooler_name.contains('\0') {
        return Err(HardwareError::InvalidConfig("spoolerName"));
    }
    if !config.width_mm.is_finite() || !(25.0..=200.0).contains(&config.width_mm) {
        return Err(HardwareError::InvalidConfig("widthMm"));
    }
    if !config.height_mm.is_finite() || !(20.0..=200.0).contains(&config.height_mm) {
        return Err(HardwareError::InvalidConfig("heightMm"));
    }
    if !config.gap_mm.is_finite() || !(0.0..=20.0).contains(&config.gap_mm) {
        return Err(HardwareError::InvalidConfig("gapMm"));
    }
    if !config.preprint_header_spacing_mm.is_finite()
        || !(0.0..=50.0).contains(&config.preprint_header_spacing_mm)
        || config.preprint_header_spacing_mm > config.height_mm - 5.0
    {
        return Err(HardwareError::InvalidConfig("preprintHeaderSpacingMm"));
    }
    if !(152..=600).contains(&config.dpi) {
        return Err(HardwareError::InvalidConfig("dpi"));
    }
    let font_sizes = [
        config.font_sizes.header,
        config.font_sizes.patient,
        config.font_sizes.withdrawal,
        config.font_sizes.drug,
        config.font_sizes.route_rate,
        config.font_sizes.storage,
        config.font_sizes.warning,
        config.font_sizes.prepared_by,
        config.font_sizes.expiration,
    ];
    if font_sizes
        .into_iter()
        .any(|value| !value.is_finite() || !(10.0..=40.0).contains(&value))
    {
        return Err(HardwareError::InvalidConfig("fontSizes"));
    }
    let width = ((config.width_mm / 25.4) * config.dpi as f32).round() as u32;
    let height = ((config.height_mm / 25.4) * config.dpi as f32).round() as u32;
    let pixels = (width as usize)
        .checked_mul(height as usize)
        .ok_or(HardwareError::InvalidConfig("dimensions"))?;
    if width == 0 || height == 0 || pixels > MAX_PIXELS {
        return Err(HardwareError::InvalidConfig("dimensions"));
    }
    Ok((width, height))
}

fn load_thai_capable_font() -> Result<FontArc, HardwareError> {
    let mut candidates = Vec::new();
    if let Ok(executable) = std::env::current_exe() {
        if let Some(directory) = executable.parent() {
            candidates.push(directory.join("fonts").join("OncoFlowThai-Regular.ttf"));
            candidates.push(
                directory
                    .join("_up_")
                    .join("resources")
                    .join("fonts")
                    .join("OncoFlowThai-Regular.ttf"),
            );
        }
    }
    if let Some(windows_directory) = std::env::var_os("WINDIR") {
        let fonts = PathBuf::from(windows_directory).join("Fonts");
        candidates.push(fonts.join("tahoma.ttf"));
        candidates.push(fonts.join("LeelawUI.ttf"));
        candidates.push(fonts.join("arial.ttf"));
    }
    for candidate in candidates {
        if let Ok(bytes) = fs::read(candidate) {
            if let Ok(font) = FontArc::try_from_vec(bytes) {
                return Ok(font);
            }
        }
    }
    Err(HardwareError::FontUnavailable)
}

fn render_label_bitmap(
    output: &PreparationOutput,
    container: &PreparationContainerLabelData,
    font: &FontArc,
    width: u32,
    height: u32,
    config: &LabelPrinterConfig,
) -> Result<MonochromeBitmap, HardwareError> {
    let label = &output.label;
    let mut bitmap = MonochromeBitmap::new(width, height);
    let margin = (width / 35).max(8);
    let available = width.saturating_sub(margin * 2);
    let base_scale = (width as f32 / 640.0).clamp(0.72, 1.7);
    let top = ((config.preprint_header_spacing_mm / 25.4) * config.dpi as f32).round() as u32;

    let header = label
        .hospital_name
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(|hospital| format!("OncoFlow - {hospital}"))
        .unwrap_or_else(|| "OncoFlow".into());
    let patient = format!(
        "{}  |  HN {}",
        display_value(label.patient_name.as_deref()),
        label.patient_identifier
    );
    let dose = [
        Some(label.drug_name.as_str()),
        label.ordered_dose_text.as_deref(),
        label.dose_unit_text.as_deref(),
    ]
    .into_iter()
    .flatten()
    .filter(|value| !value.trim().is_empty())
    .collect::<Vec<_>>()
    .join(" ");
    let diluent_volume = label
        .diluent_volume_ml
        .map(|number| format!("{} mL", format_number(number)));
    let diluent = join_display(
        label.diluent_name.as_deref(),
        diluent_volume.as_deref(),
        " ",
    );
    let drug = if diluent == "—" {
        dose
    } else {
        format!("{dose} in {diluent}")
    };
    let withdrawal = label
        .withdrawal_volume_ml
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(|value| format!("Withdrawal: {value} mL"))
        .unwrap_or_else(|| "Withdrawal: —".into());
    let rate = prefixed_rate(label.infusion_rate_or_duration.as_deref());
    let route_rate = join_display(label.route_name.as_deref(), rate.as_deref(), "  ");
    let storage = display_value(output.summary.storage_reference.as_deref()).to_owned();
    let prepared_at = label
        .prepared_at
        .as_deref()
        .and_then(|value| expiration_at(value, Some("7 hr")))
        .map(|value| format_label_datetime(&value));
    let prepared_name = label
        .prepared_by
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(|value| format!("Prepared by {value}"));
    let prepared = join_display(prepared_name.as_deref(), prepared_at.as_deref(), "  |  ");
    let expiration = format!(
        "หมดอายุ {}",
        label
            .expiration_at
            .as_deref()
            .map(format_label_datetime)
            .unwrap_or_else(|| "—".into())
    );
    let label_number = format!(
        "({}/{})",
        container.container_index,
        output.containers.len()
    );
    let expiration_fit = format!("{expiration}  {label_number}");
    let rows = vec![
        (patient, config.font_sizes.patient, 1),
        (withdrawal, config.font_sizes.withdrawal, 1),
        (drug, config.font_sizes.drug, 2),
        (route_rate, config.font_sizes.route_rate, 1),
        (storage, config.font_sizes.storage, 2),
        (
            display_value(label.warning_text.as_deref()).into(),
            config.font_sizes.warning,
            2,
        ),
        (prepared, config.font_sizes.prepared_by, 1),
        (expiration_fit, config.font_sizes.expiration, 1),
    ];
    let fit = fitted_typography_scale(
        font,
        &header,
        config.font_sizes.header,
        &rows,
        top,
        margin,
        available,
        height,
        base_scale,
    )?;
    let scale = base_scale * fit;
    let px = |value: f32| value * scale;
    let mut y = draw_centered_wrapped(
        &mut bitmap,
        font,
        &header,
        TextLayout::new(margin, top, px(config.font_sizes.header), available, 1),
    );
    y += px(6.0).ceil() as u32;
    let expiration_row = rows.len().saturating_sub(1);
    for (text, size, max_lines) in rows.into_iter().take(expiration_row) {
        y = draw_wrapped(
            &mut bitmap,
            font,
            &text,
            TextLayout::new(margin, y, px(size), available, max_lines),
        );
    }
    draw_expiration_row(
        &mut bitmap,
        font,
        &expiration,
        &label_number,
        TextLayout::new(margin, y, px(config.font_sizes.expiration), available, 1),
    );
    Ok(bitmap)
}

#[allow(clippy::too_many_arguments)]
fn fitted_typography_scale(
    font: &FontArc,
    header: &str,
    header_size: f32,
    rows: &[(String, f32, usize)],
    top: u32,
    bottom_margin: u32,
    available_width: u32,
    height: u32,
    base_scale: f32,
) -> Result<f32, HardwareError> {
    let fits = |fit: f32| {
        label_content_bottom(
            font,
            header,
            header_size,
            rows,
            top,
            available_width,
            base_scale * fit,
        ) <= height.saturating_sub(bottom_margin)
    };
    if fits(1.0) {
        return Ok(1.0);
    }
    const MINIMUM_FIT: f32 = 0.35;
    if !fits(MINIMUM_FIT) {
        return Err(HardwareError::InvalidConfig("labelContentDoesNotFit"));
    }
    let mut lower = MINIMUM_FIT;
    let mut upper = 1.0;
    for _ in 0..12 {
        let candidate = (lower + upper) / 2.0;
        if fits(candidate) {
            lower = candidate;
        } else {
            upper = candidate;
        }
    }
    Ok(lower)
}

fn label_content_bottom(
    font: &FontArc,
    header: &str,
    header_size: f32,
    rows: &[(String, f32, usize)],
    top: u32,
    available_width: u32,
    scale: f32,
) -> u32 {
    let mut y = top + wrapped_height(font, header, header_size * scale, available_width, 1);
    y += (6.0 * scale).ceil() as u32;
    for (text, size, max_lines) in rows {
        y += wrapped_height(font, text, *size * scale, available_width, *max_lines);
    }
    y
}

fn wrapped_height(
    font: &FontArc,
    text: &str,
    size: f32,
    available_width: u32,
    max_lines: usize,
) -> u32 {
    let line_count = wrap_text(font, text, size, available_width as f32, max_lines).len() as u32;
    line_count * (size * 1.35).ceil() as u32
}

fn draw_wrapped(
    bitmap: &mut MonochromeBitmap,
    font: &FontArc,
    text: &str,
    layout: TextLayout,
) -> u32 {
    let lines = wrap_text(
        font,
        text,
        layout.size,
        layout.max_width as f32,
        layout.max_lines,
    );
    let line_height = (layout.size * 1.35).ceil() as u32;
    for (index, line) in lines.iter().enumerate() {
        draw_text_line(
            bitmap,
            font,
            line,
            layout.x,
            layout.y + index as u32 * line_height,
            layout.size,
        );
    }
    layout.y + lines.len() as u32 * line_height
}

fn draw_centered_wrapped(
    bitmap: &mut MonochromeBitmap,
    font: &FontArc,
    text: &str,
    layout: TextLayout,
) -> u32 {
    let lines = wrap_text(
        font,
        text,
        layout.size,
        layout.max_width as f32,
        layout.max_lines,
    );
    let line_height = (layout.size * 1.35).ceil() as u32;
    for (index, line) in lines.iter().enumerate() {
        let width = text_width(font, line, layout.size).ceil() as u32;
        let x = layout.x + layout.max_width.saturating_sub(width) / 2;
        draw_text_line(
            bitmap,
            font,
            line,
            x,
            layout.y + index as u32 * line_height,
            layout.size,
        );
    }
    layout.y + lines.len() as u32 * line_height
}

fn draw_expiration_row(
    bitmap: &mut MonochromeBitmap,
    font: &FontArc,
    expiration: &str,
    label_number: &str,
    layout: TextLayout,
) {
    let number_width = text_width(font, label_number, layout.size).ceil() as u32;
    let gap = (layout.size * 0.5).ceil() as u32;
    let expiration_width = layout
        .max_width
        .saturating_sub(number_width.saturating_add(gap));
    draw_wrapped(
        bitmap,
        font,
        expiration,
        TextLayout::new(
            layout.x,
            layout.y,
            layout.size,
            expiration_width,
            layout.max_lines,
        ),
    );
    let number_x = layout.x + layout.max_width.saturating_sub(number_width);
    draw_text_line(bitmap, font, label_number, number_x, layout.y, layout.size);
}

fn draw_text(
    bitmap: &mut MonochromeBitmap,
    font: &FontArc,
    text: &str,
    x: u32,
    y: u32,
    size: f32,
    max_lines: usize,
) {
    let width = bitmap.width.saturating_sub(x * 2);
    draw_wrapped(
        bitmap,
        font,
        text,
        TextLayout::new(x, y, size, width, max_lines),
    );
}

#[derive(Clone, Copy)]
struct TextLayout {
    x: u32,
    y: u32,
    size: f32,
    max_width: u32,
    max_lines: usize,
}

impl TextLayout {
    const fn new(x: u32, y: u32, size: f32, max_width: u32, max_lines: usize) -> Self {
        Self {
            x,
            y,
            size,
            max_width,
            max_lines,
        }
    }
}

fn wrap_text(
    font: &FontArc,
    text: &str,
    size: f32,
    max_width: f32,
    max_lines: usize,
) -> Vec<String> {
    let scaled = font.as_scaled(PxScale::from(size));
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut width = 0.0;
    for character in text.chars() {
        let advance = scaled.h_advance(scaled.glyph_id(character));
        if !current.is_empty() && width + advance > max_width {
            lines.push(current.trim_end().to_owned());
            current.clear();
            width = 0.0;
            if lines.len() == max_lines {
                if let Some(last) = lines.last_mut() {
                    last.push('…');
                }
                return lines;
            }
        }
        current.push(character);
        width += advance;
    }
    if !current.is_empty() && lines.len() < max_lines {
        lines.push(current.trim_end().to_owned());
    }
    lines
}

fn draw_text_line(
    bitmap: &mut MonochromeBitmap,
    font: &FontArc,
    text: &str,
    x: u32,
    y: u32,
    size: f32,
) {
    let scale = PxScale::from(size);
    let scaled = font.as_scaled(scale);
    let baseline = y as f32 + scaled.ascent();
    let mut cursor = x as f32;
    let mut previous = None;
    for character in text.chars() {
        let glyph_id = scaled.glyph_id(character);
        if let Some(previous) = previous {
            cursor += scaled.kern(previous, glyph_id);
        }
        let glyph = glyph_id.with_scale_and_position(scale, point(cursor, baseline));
        if let Some(outlined) = font.outline_glyph(glyph) {
            let bounds = outlined.px_bounds();
            outlined.draw(|glyph_x, glyph_y, coverage| {
                if coverage >= 0.38 {
                    let pixel_x = bounds.min.x.floor() as i32 + glyph_x as i32;
                    let pixel_y = bounds.min.y.floor() as i32 + glyph_y as i32;
                    bitmap.set(pixel_x, pixel_y);
                }
            });
        }
        cursor += scaled.h_advance(glyph_id);
        previous = Some(glyph_id);
    }
}

fn text_width(font: &FontArc, text: &str, size: f32) -> f32 {
    let scaled = font.as_scaled(PxScale::from(size));
    let mut width = 0.0;
    let mut previous = None;
    for character in text.chars() {
        let glyph_id = scaled.glyph_id(character);
        if let Some(previous) = previous {
            width += scaled.kern(previous, glyph_id);
        }
        width += scaled.h_advance(glyph_id);
        previous = Some(glyph_id);
    }
    width.max(0.0)
}

fn encode_escpos(bitmap: &MonochromeBitmap) -> Vec<u8> {
    let width_bytes = bitmap.width_bytes();
    let mut output = vec![0x1b, 0x40, 0x1d, 0x76, 0x30, 0x00];
    output.extend_from_slice(&(width_bytes as u16).to_le_bytes());
    output.extend_from_slice(&(bitmap.height as u16).to_le_bytes());
    output.extend_from_slice(&bitmap.data);
    output.extend_from_slice(&[0x0a, 0x0a, 0x0a]);
    output
}

fn encode_tspl(bitmap: &MonochromeBitmap, config: &LabelPrinterConfig) -> Vec<u8> {
    let header = format!(
        "SIZE {:.1} mm,{:.1} mm\r\nGAP {:.1} mm,0 mm\r\nDIRECTION 1\r\nCLS\r\nBITMAP 0,0,{},{},0,",
        config.width_mm,
        config.height_mm,
        config.gap_mm,
        bitmap.width_bytes(),
        bitmap.height
    );
    let mut output = header.into_bytes();
    // The Xprinter TSPL RAW path used by OncoFlow interprets zero payload bits
    // as printed dots. Reverse the renderer's normal 1 = ink bitmap so the
    // physical label remains white and glyphs/rules print black.
    output.extend(bitmap.data.iter().map(|byte| !byte));
    output.extend_from_slice(b"\r\nPRINT 1,1\r\n");
    output
}

fn join_display(first: Option<&str>, second: Option<&str>, separator: &str) -> String {
    let joined = [first, second]
        .into_iter()
        .flatten()
        .filter(|value| !value.trim().is_empty())
        .collect::<Vec<_>>()
        .join(separator);
    if joined.is_empty() {
        "—".into()
    } else {
        joined
    }
}

fn prefixed_rate(value: Option<&str>) -> Option<String> {
    let value = value?.trim();
    if value.is_empty() || rate_starts_with_zero(value) {
        return None;
    }
    Some(format!("in {value}"))
}

fn rate_starts_with_zero(value: &str) -> bool {
    let numeric_prefix = value
        .chars()
        .take_while(|character| {
            character.is_ascii_digit() || matches!(character, '+' | '-' | '.' | ',')
        })
        .collect::<String>();
    let remainder = &value[numeric_prefix.len()..];
    !numeric_prefix.is_empty()
        && numeric_prefix
            .replace(',', ".")
            .parse::<f64>()
            .is_ok_and(|number| number == 0.0)
        && !remainder
            .chars()
            .any(|character| character.is_ascii_digit())
}

fn display_value(value: Option<&str>) -> &str {
    value
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("—")
}

fn format_label_datetime(value: &str) -> String {
    if value.len() >= 16 {
        let year = value.get(0..4);
        let month = value.get(5..7);
        let day = value.get(8..10);
        let hour = value.get(11..13);
        let minute = value.get(14..16);
        if let (Some(year), Some(month), Some(day), Some(hour), Some(minute)) =
            (year, month, day, hour, minute)
        {
            let display_year = year
                .parse::<i32>()
                .ok()
                .and_then(|year| year.checked_add(543))
                .map(|year| year.to_string())
                .unwrap_or_else(|| year.to_owned());
            return format!("{day}/{month}/{display_year} {hour}:{minute}");
        }
    }
    value.to_owned()
}

fn format_number(value: f64) -> String {
    let value = value.to_string();
    value.strip_suffix(".0").unwrap_or(&value).to_owned()
}

struct MonochromeBitmap {
    width: u32,
    height: u32,
    data: Vec<u8>,
}

impl MonochromeBitmap {
    fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            data: vec![0; (width.div_ceil(8) * height) as usize],
        }
    }

    const fn width_bytes(&self) -> u32 {
        self.width.div_ceil(8)
    }

    fn set(&mut self, x: i32, y: i32) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        let index = y as usize * self.width_bytes() as usize + x as usize / 8;
        self.data[index] |= 0x80 >> (x % 8);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::{
        PreparationContainerLabelData, PreparationLabelData, PreparationSummaryData,
    };

    fn output() -> PreparationOutput {
        PreparationOutput {
            label: PreparationLabelData {
                snapshot_id: 1,
                template_version: "oncoflow-preparation-label-v1".into(),
                generated_at: "2026-08-23T09:00:00".into(),
                print_time: "2026-08-23T09:00:00".into(),
                expiration_at: Some("2026-08-23T17:00:00".into()),
                preparation_id: 7,
                order_id: 8,
                order_reference: "OF-SYN".into(),
                patient_identifier: "SYN-HN".into(),
                patient_name: Some("ผู้ป่วยทดสอบ".into()),
                hospital_name: Some("โรงพยาบาลทดสอบ".into()),
                regimen_name: Some("สูตรทดสอบ".into()),
                treatment_at: Some("2026-08-23T09:00:00".into()),
                treatment_day: Some("Day 1".into()),
                drug_code: "SYN-D".into(),
                drug_name: "ยาเคมีบำบัดทดสอบ".into(),
                ordered_dose_text: Some("100".into()),
                dose_unit_text: Some("mg".into()),
                diluent_name: Some("สารละลายทดสอบ".into()),
                diluent_volume_ml: Some(100.0),
                withdrawal_volume_ml: Some("20".into()),
                final_volume_ml: Some(120.0),
                route_name: Some("IV".into()),
                infusion_rate_or_duration: Some("60 min".into()),
                warning_text: Some("คำเตือนทดสอบ".into()),
                expiry_time_text: Some("8 hr".into()),
                expiry_storage_text: Some("ป้องกันแสง".into()),
                prepared_by: Some("เภสัชกรหนึ่ง".into()),
                prepared_at: Some("2026-08-23T09:10:00".into()),
                verified_by: Some("เภสัชกรสอง".into()),
                verified_at: "2026-08-23T09:15:00".into(),
            },
            containers: vec![PreparationContainerLabelData { container_index: 1 }],
            summary: PreparationSummaryData {
                preparation_instructions: None,
                preparation_notes: None,
                storage_reference: None,
                safety_review_status: "verified_workflow_complete",
                inventory_posting_status: Some("posted".into()),
                inventory_movement_id: Some(1),
                containers_required: Some(2),
                inventory_balance_before: Some(1.0),
                inventory_balance_after: Some(-1.0),
                inventory_stock_state: Some("shortage".into()),
                calculation_ruleset_version: Some("legacy-cytotoxic-v8".into()),
                calculation_rule_id: Some("synthetic".into()),
                presentation_notice: "Persisted values only.",
            },
            print_request_count: 0,
        }
    }

    fn config(language: PrinterLanguage) -> LabelPrinterConfig {
        LabelPrinterConfig {
            spooler_name: "Synthetic queue".into(),
            language,
            width_mm: 100.0,
            height_mm: 70.0,
            dpi: 203,
            gap_mm: 3.0,
            preprint_header_spacing_mm: 5.0,
            font_sizes: Default::default(),
        }
    }

    #[test]
    fn renders_deterministic_thai_escpos_raster_bytes() {
        let first = render_preparation_label(&output(), &config(PrinterLanguage::Escpos)).unwrap();
        let second = render_preparation_label(&output(), &config(PrinterLanguage::Escpos)).unwrap();
        assert_eq!(first, second);
        assert!(first.starts_with(&[0x1b, 0x40, 0x1d, 0x76, 0x30, 0x00]));
        assert!(first.iter().any(|byte| *byte != 0));
    }

    #[test]
    fn renders_tspl_bitmap_without_plain_clinical_text() {
        let bytes = render_preparation_label(&output(), &config(PrinterLanguage::Tspl)).unwrap();
        assert!(bytes.starts_with(b"SIZE 100.0 mm,70.0 mm\r\n"));
        assert!(bytes.ends_with(b"\r\nPRINT 1,1\r\n"));
        let printable = String::from_utf8_lossy(&bytes);
        assert!(!printable.contains("SYN-HN"));
        assert!(!printable.contains("ผู้ป่วย"));
    }

    #[test]
    fn renders_one_complete_label_command_per_batch_item() {
        let bytes =
            render_preparation_labels(&[output(), output()], &config(PrinterLanguage::Tspl))
                .unwrap();
        assert_eq!(
            bytes
                .windows(b"\r\nPRINT 1,1\r\n".len())
                .filter(|window| *window == b"\r\nPRINT 1,1\r\n")
                .count(),
            2
        );
    }

    #[test]
    fn renders_one_complete_label_for_every_final_container() {
        let mut multi = output();
        multi.containers = vec![
            PreparationContainerLabelData { container_index: 1 },
            PreparationContainerLabelData { container_index: 2 },
        ];
        let bytes = render_preparation_label(&multi, &config(PrinterLanguage::Tspl)).unwrap();
        assert_eq!(
            bytes
                .windows(b"\r\nPRINT 1,1\r\n".len())
                .filter(|window| *window == b"\r\nPRINT 1,1\r\n")
                .count(),
            2
        );
    }

    #[test]
    fn auto_fits_complete_content_into_an_eighty_by_fifty_label() {
        let mut compact = config(PrinterLanguage::Tspl);
        compact.width_mm = 80.0;
        compact.height_mm = 50.0;
        compact.font_sizes.header = 32.0;
        compact.font_sizes.patient = 32.0;
        compact.font_sizes.drug = 32.0;
        let bytes = render_preparation_label(&output(), &compact).unwrap();
        assert!(bytes.starts_with(b"SIZE 80.0 mm,50.0 mm\r\n"));
        assert!(bytes.ends_with(b"\r\nPRINT 1,1\r\n"));
    }

    #[test]
    fn reverses_tspl_raster_for_white_background_and_black_text() {
        let blank = MonochromeBitmap::new(8, 1);
        let blank_bytes = encode_tspl(&blank, &config(PrinterLanguage::Tspl));
        let blank_payload = tspl_payload(&blank_bytes, blank.data.len());
        assert_eq!(blank_payload, &[0xff]);

        let mut marked = MonochromeBitmap::new(8, 1);
        marked.set(0, 0);
        let marked_bytes = encode_tspl(&marked, &config(PrinterLanguage::Tspl));
        let marked_payload = tspl_payload(&marked_bytes, marked.data.len());
        assert_eq!(marked_payload, &[0x7f]);
    }

    #[test]
    fn rejects_unsafe_or_unbounded_device_configuration() {
        let mut invalid = config(PrinterLanguage::Tspl);
        invalid.spooler_name = "bad\0queue".into();
        assert!(matches!(
            render_preparation_label(&output(), &invalid),
            Err(HardwareError::InvalidConfig("spoolerName"))
        ));
        invalid = config(PrinterLanguage::Tspl);
        invalid.dpi = 5_000;
        assert!(matches!(
            render_preparation_label(&output(), &invalid),
            Err(HardwareError::InvalidConfig("dpi"))
        ));
        invalid = config(PrinterLanguage::Tspl);
        invalid.font_sizes.warning = 41.0;
        assert!(matches!(
            render_preparation_label(&output(), &invalid),
            Err(HardwareError::InvalidConfig("fontSizes"))
        ));
        invalid = config(PrinterLanguage::Tspl);
        invalid.preprint_header_spacing_mm = 80.0;
        assert!(matches!(
            render_preparation_label(&output(), &invalid),
            Err(HardwareError::InvalidConfig("preprintHeaderSpacingMm"))
        ));
        invalid = config(PrinterLanguage::Tspl);
        invalid.height_mm = 50.0;
        invalid.preprint_header_spacing_mm = 45.0;
        assert!(matches!(
            render_preparation_label(&output(), &invalid),
            Err(HardwareError::InvalidConfig("labelContentDoesNotFit"))
        ));
    }

    #[test]
    fn omits_rate_prefix_for_missing_or_zero_values() {
        assert_eq!(prefixed_rate(None), None);
        assert_eq!(prefixed_rate(Some("  ")), None);
        assert_eq!(prefixed_rate(Some("0")), None);
        assert_eq!(prefixed_rate(Some("0.0 min")), None);
        assert_eq!(prefixed_rate(Some("0,0 mL/hr")), None);
        assert_eq!(prefixed_rate(Some("00:30")).as_deref(), Some("in 00:30"));
        assert_eq!(prefixed_rate(Some("60 min")).as_deref(), Some("in 60 min"));
    }

    fn tspl_payload(bytes: &[u8], payload_length: usize) -> &[u8] {
        const TRAILER: &[u8] = b"\r\nPRINT 1,1\r\n";
        assert!(bytes.ends_with(TRAILER));
        let payload_end = bytes.len() - TRAILER.len();
        &bytes[payload_end - payload_length..payload_end]
    }
}
