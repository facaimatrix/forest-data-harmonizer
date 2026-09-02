//! Lightweight export/UI string localization (en / es / pt).

pub fn normalize_locale(locale: &str) -> &str {
    match locale.split('-').next().unwrap_or("en").to_lowercase().as_str() {
        "es" => "es",
        "pt" => "pt",
        _ => "en",
    }
}

/// Curation-log section headers and field labels.
pub fn curation_label(locale: &str, key: &str) -> String {
    let loc = normalize_locale(locale);
    match (loc, key) {
        ("es", "dataset") => "CONJUNTO:".into(),
        ("es", "country") => "PAÍS:".into(),
        ("es", "site") => "SITIO:".into(),
        ("es", "pi") => "PI:".into(),
        ("es", "curator") => "CURADOR:".into(),
        ("es", "date_received") => "FECHA RECIBIDA:".into(),
        ("es", "date_processed") => "FECHA PROCESADA:".into(),
        ("es", "source") => "--- FORMATO ORIGEN ---".into(),
        ("es", "pivot") => "--- PIVOT / REESTRUCTURACIÓN ---".into(),
        ("es", "duplicate") => "--- RESOLUCIÓN DE DUPLICADOS ---".into(),
        ("es", "missing") => "--- DATOS FALTANTES / INTERPOLADOS ---".into(),
        ("es", "species") => "--- PROBLEMAS DE ESPECIES ---".into(),
        ("es", "exclusions") => "--- EXCLUSIONES ---".into(),
        ("es", "notes") => "--- NOTAS ---".into(),
        ("es", "auto_flagged") => "[AUTO-MARCADO]".into(),

        ("pt", "dataset") => "CONJUNTO:".into(),
        ("pt", "country") => "PAÍS:".into(),
        ("pt", "site") => "SÍTIO:".into(),
        ("pt", "pi") => "PI:".into(),
        ("pt", "curator") => "CURADOR:".into(),
        ("pt", "date_received") => "DATA RECEBIDA:".into(),
        ("pt", "date_processed") => "DATA PROCESSADA:".into(),
        ("pt", "source") => "--- FORMATO ORIGEM ---".into(),
        ("pt", "pivot") => "--- PIVOT / REESTRUTURAÇÃO ---".into(),
        ("pt", "duplicate") => "--- RESOLUÇÃO DE DUPLICADOS ---".into(),
        ("pt", "missing") => "--- DADOS FALTANTES / INTERPOLADOS ---".into(),
        ("pt", "species") => "--- PROBLEMAS DE ESPÉCIES ---".into(),
        ("pt", "exclusions") => "--- EXCLUSÕES ---".into(),
        ("pt", "notes") => "--- NOTAS ---".into(),
        ("pt", "auto_flagged") => "[AUTO-MARCADO]".into(),

        (_, "dataset") => "DATASET:".into(),
        (_, "country") => "COUNTRY:".into(),
        (_, "site") => "SITE:".into(),
        (_, "pi") => "PI:".into(),
        (_, "curator") => "CURATOR:".into(),
        (_, "date_received") => "DATE RECEIVED:".into(),
        (_, "date_processed") => "DATE PROCESSED:".into(),
        (_, "source") => "--- SOURCE FORMAT ---".into(),
        (_, "pivot") => "--- PIVOT / RESTRUCTURING ---".into(),
        (_, "duplicate") => "--- DUPLICATE RESOLUTION ---".into(),
        (_, "missing") => "--- MISSING / INTERPOLATED DATA ---".into(),
        (_, "species") => "--- SPECIES ISSUES ---".into(),
        (_, "exclusions") => "--- EXCLUSIONS ---".into(),
        (_, "notes") => "--- NOTES ---".into(),
        (_, "auto_flagged") => "[AUTO-FLAGGED]".into(),
        _ => key.to_string(),
    }
}

/// Diagnostic report strings (HTML/PDF headings and labels).
pub fn diagnostic_label(locale: &str, key: &str) -> String {
    let loc = normalize_locale(locale);
    match (loc, key) {
        ("es", "title_multi") => "Informe diagnóstico de formato GFB3".into(),
        ("es", "title_single") => "Informe diagnóstico GFB2 / censo único".into(),
        ("es", "overview") => "Resumen".into(),
        ("es", "status_distribution") => "Distribución de estado".into(),
        ("es", "dbh_summary") => "Resumen DBH".into(),
        ("es", "growth_summary") => "Resumen de crecimiento".into(),
        ("es", "basal_area") => "Área basal".into(),
        ("es", "tph") => "Árboles por hectárea".into(),
        ("es", "flags") => "Indicadores de calidad de datos".into(),
        ("es", "curation_notes") => "Notas de curación".into(),
        ("es", "mode_multi") => "Multi-censo (GFB3)".into(),
        ("es", "mode_single") => "Censo único (GFB2)".into(),
        ("es", "growth_na") => "No aplicable para censo único (GFB2).".into(),
        ("es", "dataset") => "Conjunto".into(),
        ("es", "mode") => "Modo".into(),
        ("es", "rows") => "Filas".into(),
        ("es", "trees") => "Árboles".into(),
        ("es", "plots") => "Parcelas".into(),
        ("es", "year_range") => "Rango de años".into(),
        ("es", "th_status") => "Estado".into(),
        ("es", "th_label") => "Etiqueta".into(),
        ("es", "th_n") => "n".into(),
        ("es", "th_pct") => "%".into(),
        ("es", "th_flag") => "Indicador".into(),
        ("es", "th_count") => "Conteo".into(),
        ("es", "th_severity") => "Gravedad".into(),
        ("es", "verdict_heading") => "VEREDICTO".into(),
        ("es", "growth_paired") => "Tallos vivos emparejados".into(),
        ("es", "growth_mean_delta") => "Media ΔDBH".into(),
        ("es", "growth_mean_annual") => "Crecimiento anual medio".into(),
        ("es", "growth_pct_neg") => "Crecimiento negativo".into(),
        ("es", "growth_zero") => "Crecimiento cero".into(),
        ("es", "growth_fast") => "Anual > 5 cm/año".into(),
        ("es", "dbh_n") => "n".into(),
        ("es", "dbh_mean_sd") => "media ± desv".into(),
        ("es", "dbh_quantiles") => "mín / Q25 / mediana / Q75 / máx".into(),
        ("es", "chart_dbh_hist") => "Distribución DBH (cm)".into(),
        ("es", "chart_growth_hist") => "Crecimiento DBH Δ (cm)".into(),
        ("es", "chart_ba_by_plot") => "Área basal por parcela × censo".into(),
        ("es", "chart_tph_by_plot") => "Árboles por hectárea por parcela × censo".into(),
        ("es", "chart_ba_ylabel") => "AB (m²/ha)".into(),
        ("es", "chart_tph_ylabel") => "APH".into(),
        ("es", "pdf_diagnostic") => "Diagnóstico".into(),

        ("pt", "title_multi") => "Relatório diagnóstico de formato GFB3".into(),
        ("pt", "title_single") => "Relatório diagnóstico GFB2 / censo único".into(),
        ("pt", "overview") => "Visão geral".into(),
        ("pt", "status_distribution") => "Distribuição de status".into(),
        ("pt", "dbh_summary") => "Resumo DBH".into(),
        ("pt", "growth_summary") => "Resumo de crescimento".into(),
        ("pt", "basal_area") => "Área basal".into(),
        ("pt", "tph") => "Árvores por hectare".into(),
        ("pt", "flags") => "Indicadores de qualidade dos dados".into(),
        ("pt", "curation_notes") => "Notas de curadoria".into(),
        ("pt", "mode_multi") => "Multi-censo (GFB3)".into(),
        ("pt", "mode_single") => "Censo único (GFB2)".into(),
        ("pt", "growth_na") => "Não aplicável para censo único (GFB2).".into(),
        ("pt", "dataset") => "Conjunto".into(),
        ("pt", "mode") => "Modo".into(),
        ("pt", "rows") => "Linhas".into(),
        ("pt", "trees") => "Árvores".into(),
        ("pt", "plots") => "Parcelas".into(),
        ("pt", "year_range") => "Intervalo de anos".into(),
        ("pt", "th_status") => "Status".into(),
        ("pt", "th_label") => "Rótulo".into(),
        ("pt", "th_n") => "n".into(),
        ("pt", "th_pct") => "%".into(),
        ("pt", "th_flag") => "Indicador".into(),
        ("pt", "th_count") => "Contagem".into(),
        ("pt", "th_severity") => "Gravidade".into(),
        ("pt", "verdict_heading") => "VEREDITO".into(),
        ("pt", "growth_paired") => "Indivíduos vivos pareados".into(),
        ("pt", "growth_mean_delta") => "Média ΔDBH".into(),
        ("pt", "growth_mean_annual") => "Crescimento anual médio".into(),
        ("pt", "growth_pct_neg") => "Crescimento negativo".into(),
        ("pt", "growth_zero") => "Crescimento zero".into(),
        ("pt", "growth_fast") => "Anual > 5 cm/ano".into(),
        ("pt", "dbh_n") => "n".into(),
        ("pt", "dbh_mean_sd") => "média ± desv".into(),
        ("pt", "dbh_quantiles") => "mín / Q25 / mediana / Q75 / máx".into(),
        ("pt", "chart_dbh_hist") => "Distribuição DBH (cm)".into(),
        ("pt", "chart_growth_hist") => "Crescimento DBH Δ (cm)".into(),
        ("pt", "chart_ba_by_plot") => "Área basal por parcela × censo".into(),
        ("pt", "chart_tph_by_plot") => "Árvores por hectare por parcela × censo".into(),
        ("pt", "chart_ba_ylabel") => "AB (m²/ha)".into(),
        ("pt", "chart_tph_ylabel") => "APH".into(),
        ("pt", "pdf_diagnostic") => "Diagnóstico".into(),

        (_, "title_multi") => "GFB3 Format Diagnostic Report".into(),
        (_, "title_single") => "GFB2 / Single-census Diagnostic Report".into(),
        (_, "overview") => "Overview".into(),
        (_, "status_distribution") => "Status distribution".into(),
        (_, "dbh_summary") => "DBH summary".into(),
        (_, "growth_summary") => "Growth summary".into(),
        (_, "basal_area") => "Basal area".into(),
        (_, "tph") => "Trees per hectare".into(),
        (_, "flags") => "Data quality flags".into(),
        (_, "curation_notes") => "Curation notes".into(),
        (_, "mode_multi") => "Multi-census (GFB3)".into(),
        (_, "mode_single") => "Single-census (GFB2)".into(),
        (_, "growth_na") => "Not applicable for single-census (GFB2) data.".into(),
        (_, "dataset") => "Dataset".into(),
        (_, "mode") => "Mode".into(),
        (_, "rows") => "Rows".into(),
        (_, "trees") => "Trees".into(),
        (_, "plots") => "Plots".into(),
        (_, "year_range") => "Year range".into(),
        (_, "th_status") => "Status".into(),
        (_, "th_label") => "Label".into(),
        (_, "th_n") => "n".into(),
        (_, "th_pct") => "%".into(),
        (_, "th_flag") => "Flag".into(),
        (_, "th_count") => "Count".into(),
        (_, "th_severity") => "Severity".into(),
        (_, "verdict_heading") => "VERDICT".into(),
        (_, "growth_paired") => "Paired alive stems".into(),
        (_, "growth_mean_delta") => "Mean ΔDBH".into(),
        (_, "growth_mean_annual") => "Mean annual growth".into(),
        (_, "growth_pct_neg") => "Negative growth".into(),
        (_, "growth_zero") => "Zero growth".into(),
        (_, "growth_fast") => "Annual > 5 cm/yr".into(),
        (_, "dbh_n") => "n".into(),
        (_, "dbh_mean_sd") => "mean ± sd".into(),
        (_, "dbh_quantiles") => "min / Q25 / median / Q75 / max".into(),
        (_, "chart_dbh_hist") => "DBH distribution (cm)".into(),
        (_, "chart_growth_hist") => "DBH growth Δ (cm)".into(),
        (_, "chart_ba_by_plot") => "Basal area by plot × census".into(),
        (_, "chart_tph_by_plot") => "Trees per hectare by plot × census".into(),
        (_, "chart_ba_ylabel") => "BA (m²/ha)".into(),
        (_, "chart_tph_ylabel") => "TPH".into(),
        (_, "pdf_diagnostic") => "Diagnostic".into(),
        _ => key.to_string(),
    }
}

pub fn diagnostic_verdict(locale: &str, level: &str) -> String {
    let loc = normalize_locale(locale);
    match (loc, level) {
        ("es", "fail") => "FALLO: hay problemas críticos que deben resolverse antes del uso.".into(),
        ("es", "warn") => {
            "ADVERTENCIA: pasó las comprobaciones críticas pero tiene avisos que conviene revisar.".into()
        }
        ("es", "pass") => "APROBADO: el conjunto parece limpio.".into(),
        ("pt", "fail") => "FALHA: problemas críticos devem ser resolvidos antes do uso.".into(),
        ("pt", "warn") => {
            "AVISO: passou nas verificações críticas, mas há alertas que valem revisão.".into()
        }
        ("pt", "pass") => "APROVADO: o conjunto parece limpo.".into(),
        (_, "fail") => "FAIL: critical issues must be resolved before use.".into(),
        (_, "warn") => "WARN: passed critical checks but has warnings worth reviewing.".into(),
        (_, "pass") => "PASS: dataset looks clean.".into(),
        _ => level.to_string(),
    }
}

pub fn diagnostic_flag(locale: &str, key: &str) -> String {
    let loc = normalize_locale(locale);
    match (loc, key) {
        ("es", "missing_yr") => "YR faltante".into(),
        ("es", "missing_dbh") => "DBH faltante (árboles muertos excluidos)".into(),
        ("es", "missing_species") => "Especie faltante o mal formada".into(),
        ("es", "dbh_small") => "DBH < 10 cm".into(),
        ("es", "duplicate") => "TreeID × YR duplicado".into(),
        ("es", "ba_warning") => "AB 51–100 m²/ha (más alto de lo típico)".into(),
        ("es", "ba_critical") => "AB ≥ 100 m²/ha (inverosímil)".into(),
        ("es", "missing_prevdbh") => "PrevDBH faltante (reclutas excluidos)".into(),
        ("es", "growth_negative") => "Crecimiento DBH negativo (%)".into(),
        ("es", "growth_zero") => "Crecimiento DBH cero".into(),
        ("es", "growth_fast") => "Crecimiento anual > 5 cm/año".into(),
        ("es", "zombie") => "Árboles zombi (vivos tras muerte)".into(),
        ("es", "prevdbh_mismatch") => "PrevDBH no coincide con rezago(DBH)".into(),
        ("es", "prevyear_mismatch") => "PrevYR no coincide con rezago(YR)".into(),
        ("es", "prevdbh_orphan") => "PrevDBH faltante pero existe rezago(DBH) válido (vivo)".into(),
        ("es", "prevyear_orphan") => "PrevYR faltante pero existe rezago(YR) válido (vivo)".into(),

        ("pt", "missing_yr") => "YR ausente".into(),
        ("pt", "missing_dbh") => "DBH ausente (mortos excluídos)".into(),
        ("pt", "missing_species") => "Espécie ausente ou malformada".into(),
        ("pt", "dbh_small") => "DBH < 10 cm".into(),
        ("pt", "duplicate") => "TreeID × YR duplicado".into(),
        ("pt", "ba_warning") => "AB 51–100 m²/ha (acima do típico)".into(),
        ("pt", "ba_critical") => "AB ≥ 100 m²/ha (implausível)".into(),
        ("pt", "missing_prevdbh") => "PrevDBH ausente (recrutas excluídos)".into(),
        ("pt", "growth_negative") => "Crescimento DBH negativo (%)".into(),
        ("pt", "growth_zero") => "Crescimento DBH zero".into(),
        ("pt", "growth_fast") => "Crescimento anual > 5 cm/ano".into(),
        ("pt", "zombie") => "Árvores zumbi (vivas após morte)".into(),
        ("pt", "prevdbh_mismatch") => "PrevDBH não coincide com defasagem(DBH)".into(),
        ("pt", "prevyear_mismatch") => "PrevYR não coincide com defasagem(YR)".into(),
        ("pt", "prevdbh_orphan") => "PrevDBH ausente mas defasagem(DBH) válida existe (vivo)".into(),
        ("pt", "prevyear_orphan") => "PrevYR ausente mas defasagem(YR) válida existe (vivo)".into(),

        (_, "missing_yr") => "Missing YR".into(),
        (_, "missing_dbh") => "Missing DBH (dead trees excluded)".into(),
        (_, "missing_species") => "Missing/malformed Species".into(),
        (_, "dbh_small") => "DBH < 10 cm".into(),
        (_, "duplicate") => "Duplicate TreeID × YR".into(),
        (_, "ba_warning") => "BA 51–100 m²/ha (higher than typical)".into(),
        (_, "ba_critical") => "BA ≥ 100 m²/ha (implausible)".into(),
        (_, "missing_prevdbh") => "Missing PrevDBH (recruits excluded)".into(),
        (_, "growth_negative") => "Negative DBH growth (%)".into(),
        (_, "growth_zero") => "Zero DBH growth".into(),
        (_, "growth_fast") => "Annual growth > 5 cm/yr".into(),
        (_, "zombie") => "Zombie trees (alive after death)".into(),
        (_, "prevdbh_mismatch") => "PrevDBH value mismatches lag(DBH)".into(),
        (_, "prevyear_mismatch") => "PrevYR value mismatches lag(YR)".into(),
        (_, "prevdbh_orphan") => "PrevDBH missing but valid lag(DBH) exists (alive)".into(),
        (_, "prevyear_orphan") => "PrevYR missing but valid lag(YR) exists (alive)".into(),
        _ => key.to_string(),
    }
}

pub fn diagnostic_skip_reason(locale: &str, key: &str) -> String {
    let loc = normalize_locale(locale);
    match (loc, key) {
        ("es", "no_data") => "sin datos".into(),
        ("es", "dbh_missing") => "columna DBH faltante".into(),
        ("es", "pa_missing") => {
            "PA (área de parcela, ha) no asignada — asigne columna o constante en Mapeo de columnas".into()
        }
        ("es", "no_alive_dbh") => "no hay tallos vivos con DBH y PA > 0 válidos".into(),
        ("es", "ba_aggregate_fail") => "no se pudo agregar AB/APH".into(),
        ("pt", "no_data") => "sem dados".into(),
        ("pt", "dbh_missing") => "coluna DBH ausente".into(),
        ("pt", "pa_missing") => {
            "PA (área da parcela, ha) não mapeada — atribua coluna ou constante em Mapeamento de colunas".into()
        }
        ("pt", "no_alive_dbh") => "nenhum indivíduo vivo com DBH e PA > 0 válidos".into(),
        ("pt", "ba_aggregate_fail") => "não foi possível agregar AB/APH".into(),
        (_, "no_data") => "no data".into(),
        (_, "dbh_missing") => "DBH column missing".into(),
        (_, "pa_missing") => {
            "PA (plot area, ha) not mapped — assign a column or constant in Field assignment".into()
        }
        (_, "no_alive_dbh") => "no alive stems with valid DBH and PA > 0".into(),
        (_, "ba_aggregate_fail") => "could not aggregate BA/TPH".into(),
        _ => key.to_string(),
    }
}

pub fn severity_label(locale: &str, severity: &str) -> String {
    let loc = normalize_locale(locale);
    match (loc, severity) {
        ("es", "critical") => "crítico".into(),
        ("es", "warning") => "advertencia".into(),
        ("es", "info") => "info".into(),
        ("pt", "critical") => "crítico".into(),
        ("pt", "warning") => "aviso".into(),
        ("pt", "info") => "info".into(),
        (_, "critical") => "critical".into(),
        (_, "warning") => "warning".into(),
        (_, "info") => "info".into(),
        _ => severity.to_string(),
    }
}

pub fn status_label(locale: &str, code: &str) -> &'static str {
    let loc = normalize_locale(locale);
    match (loc, code) {
        ("es", "0") => "vivo",
        ("es", "1") => "muerto entre inventarios",
        ("es", "2") => "recluta nuevo",
        ("es", "9") => "ausente",
        ("pt", "0") => "vivo",
        ("pt", "1") => "morto entre inventários",
        ("pt", "2") => "recruta novo",
        ("pt", "9") => "ausente",
        (_, "0") => "alive",
        (_, "1") => "dead between inventories",
        (_, "2") => "new recruit",
        (_, "9") => "missing",
        _ => "unknown",
    }
}

pub fn chart_no_data(locale: &str) -> &'static str {
    match normalize_locale(locale) {
        "es" => " — sin datos",
        "pt" => " — sem dados",
        _ => " — no data",
    }
}

pub fn unidentified_species(locale: &str) -> &'static str {
    match normalize_locale(locale) {
        "es" => "Esp. no identificada",
        "pt" => "Esp. não identificada",
        _ => "Unidentified sp.",
    }
}
