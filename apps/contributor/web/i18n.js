// GFB3 Harmonizer — lightweight i18n (no bundler required)
(function (global) {
  'use strict';

  const STORAGE_KEY = 'gfb3-harmonizer-locale';

  const LOCALE_META = [
    { code: 'en', label: 'English', native: 'English', dir: 'ltr' },
    { code: 'es', label: 'Spanish', native: 'Español', dir: 'ltr' },
    { code: 'pt', label: 'Portuguese', native: 'Português', dir: 'ltr' },
    { code: 'fr', label: 'French', native: 'Français', dir: 'ltr' },
    { code: 'de', label: 'German', native: 'Deutsch', dir: 'ltr' },
    { code: 'it', label: 'Italian', native: 'Italiano', dir: 'ltr' },
    { code: 'nl', label: 'Dutch', native: 'Nederlands', dir: 'ltr' },
    { code: 'ru', label: 'Russian', native: 'Русский', dir: 'ltr' },
    { code: 'ja', label: 'Japanese', native: '日本語', dir: 'ltr' },
    { code: 'zh', label: 'Chinese', native: '中文', dir: 'ltr' },
    { code: 'ko', label: 'Korean', native: '한국어', dir: 'ltr' },
    { code: 'hi', label: 'Hindi', native: 'हिन्दी', dir: 'ltr' },
    { code: 'id', label: 'Indonesian', native: 'Bahasa Indonesia', dir: 'ltr' },
    { code: 'th', label: 'Thai', native: 'ไทย', dir: 'ltr' },
    { code: 'ar', label: 'Arabic', native: 'العربية', dir: 'rtl' },
  ];

  const ISO3_TO_ISO2 = {
    AFG:'AF',ALB:'AL',DZA:'DZ',AGO:'AO',ARG:'AR',ARM:'AM',AUS:'AU',AUT:'AT',AZE:'AZ',
    BGD:'BD',BLR:'BY',BEL:'BE',BLZ:'BZ',BEN:'BJ',BTN:'BT',BOL:'BO',BIH:'BA',BWA:'BW',
    BRA:'BR',BRN:'BN',BGR:'BG',BFA:'BF',BDI:'BI',CPV:'CV',KHM:'KH',CMR:'CM',CAF:'CF',
    TCD:'TD',CHL:'CL',CHN:'CN',COL:'CO',COM:'KM',COG:'CG',COD:'CD',CRI:'CR',CIV:'CI',
    HRV:'HR',CUB:'CU',CZE:'CZ',DNK:'DK',DJI:'DJ',DOM:'DO',ECU:'EC',EGY:'EG',SLV:'SV',
    GNQ:'GQ',ERI:'ER',EST:'EE',SWZ:'SZ',ETH:'ET',FJI:'FJ',FIN:'FI',FRA:'FR',GUF:'GF',
    GAB:'GA',GMB:'GM',GEO:'GE',DEU:'DE',GHA:'GH',GRC:'GR',GTM:'GT',GIN:'GN',GNB:'GW',
    GUY:'GY',HTI:'HT',HND:'HN',HUN:'HU',IND:'IN',IDN:'ID',IRN:'IR',IRQ:'IQ',IRL:'IE',
    ISR:'IL',ITA:'IT',JAM:'JM',JPN:'JP',JOR:'JO',KAZ:'KZ',KEN:'KE',KGZ:'KG',LAO:'LA',
    LVA:'LV',LBN:'LB',LSO:'LS',LBR:'LR',LBY:'LY',LTU:'LT',LUX:'LU',MDG:'MG',MWI:'MW',
    MYS:'MY',MLI:'ML',MRT:'MR',MUS:'MU',MEX:'MX',MDA:'MD',MNG:'MN',MAR:'MA',MOZ:'MZ',
    MMR:'MM',NAM:'NA',NPL:'NP',NLD:'NL',NZL:'NZ',NIC:'NI',NER:'NE',NGA:'NG',NOR:'NO',
    PAK:'PK',PAN:'PA',PNG:'PG',PRY:'PY',PER:'PE',PHL:'PH',POL:'PL',PRT:'PT',ROU:'RO',
    RUS:'RU',RWA:'RW',STP:'ST',SEN:'SN',SLE:'SL',SGP:'SG',SLB:'SB',SOM:'SO',ZAF:'ZA',
    SSD:'SS',ESP:'ES',LKA:'LK',SDN:'SD',SUR:'SR',SWE:'SE',CHE:'CH',TWN:'TW',TJK:'TJ',
    TZA:'TZ',THA:'TH',TLS:'TL',TGO:'TG',TTO:'TT',TUN:'TN',TUR:'TR',TKM:'TM',UGA:'UG',
    UKR:'UA',GBR:'GB',USA:'US',URY:'UY',UZB:'UZ',VEN:'VE',VNM:'VN',YEM:'YE',ZMB:'ZM',
    ZWE:'ZW',
  };

  let currentLocale = 'en';
  let onChange = null;

  function normalizeLocale(code) {
    if (!code) return 'en';
    const base = String(code).trim().toLowerCase().replace('_', '-').split('-')[0];
    return LOCALE_META.some(m => m.code === base) ? base : 'en';
  }

  function detectLocale() {
    try {
      const stored = localStorage.getItem(STORAGE_KEY);
      if (stored) return normalizeLocale(stored);
    } catch (_) { /* private mode */ }
    const langs = navigator.languages && navigator.languages.length
      ? navigator.languages
      : [navigator.language || 'en'];
    for (const lang of langs) {
      const n = normalizeLocale(lang);
      if (n !== 'en' || String(lang).toLowerCase().startsWith('en')) return n;
      if (LOCALE_META.some(m => m.code === n)) return n;
    }
    return 'en';
  }

  function getCatalog(locale) {
    const packs = global.I18N_LOCALES || {};
    return packs[locale] || packs.en || {};
  }

  function interpolate(str, params) {
    if (!params || str == null) return str;
    return String(str).replace(/\{(\w+)\}/g, (_, k) =>
      params[k] != null ? String(params[k]) : `{${k}}`);
  }

  function t(key, params) {
    const catalog = getCatalog(currentLocale);
    const fallback = getCatalog('en');
    const raw = catalog[key] ?? fallback[key] ?? key;
    return interpolate(raw, params);
  }

  function setLocale(code, opts = {}) {
    const next = normalizeLocale(code);
    if (next === currentLocale && !opts.force) return;
    currentLocale = next;
    try { localStorage.setItem(STORAGE_KEY, next); } catch (_) { /* ignore */ }
    const meta = LOCALE_META.find(m => m.code === next) || LOCALE_META[0];
    document.documentElement.lang = next;
    document.documentElement.dir = meta.dir || 'ltr';
    document.title = t('meta.title');
    applyStaticLabels();
    renderLanguageSelector('lang-switcher');
    if (typeof onChange === 'function' && !opts.silent) onChange(next);
  }

  function getLocale() { return currentLocale; }

  function getLocales() { return LOCALE_META.slice(); }

  function countryName(iso3, englishFallback) {
    const iso2 = ISO3_TO_ISO2[String(iso3 || '').toUpperCase()];
    if (iso2) {
      try {
        const dn = new Intl.DisplayNames([currentLocale], { type: 'region' });
        const localized = dn.of(iso2);
        if (localized) return localized;
      } catch (_) { /* Intl unavailable */ }
    }
    return englishFallback || iso3;
  }

  function applyStaticLabels() {
    document.querySelectorAll('[data-i18n]').forEach(el => {
      // Skip containers that hold dynamic app content (textContent would wipe children).
      if (el.id === 'main') return;
      const key = el.getAttribute('data-i18n');
      if (key) el.textContent = t(key);
    });
    document.querySelectorAll('[data-i18n-placeholder]').forEach(el => {
      const key = el.getAttribute('data-i18n-placeholder');
      if (key) el.placeholder = t(key);
    });
    const sel = document.getElementById('lang-select');
    if (sel) sel.value = currentLocale;
    const hdrTitle = document.getElementById('header-title');
    const hdrSub = document.getElementById('header-subtitle');
    if (hdrTitle) hdrTitle.textContent = t('header.title');
    if (hdrSub) hdrSub.textContent = t('header.subtitle');
  }

  function renderLanguageSelector(containerId) {
    const host = document.getElementById(containerId);
    if (!host) return;
    const options = LOCALE_META.map(m =>
      `<option value="${m.code}" ${m.code === currentLocale ? 'selected' : ''}>${m.native}</option>`
    ).join('');
    host.innerHTML = `
      <label class="lang-label" for="lang-select" data-i18n="header.language">${t('header.language')}</label>
      <select id="lang-select" class="lang-select" aria-label="${t('header.language')}">${options}</select>`;
    const sel = document.getElementById('lang-select');
    if (sel) {
      sel.addEventListener('change', e => {
        setLocale(e.target.value);
      });
    }
  }

  function init(options = {}) {
    onChange = options.onChange || null;
    currentLocale = detectLocale();
    document.documentElement.lang = currentLocale;
    const meta = LOCALE_META.find(m => m.code === currentLocale) || LOCALE_META[0];
    document.documentElement.dir = meta.dir || 'ltr';
    renderLanguageSelector('lang-switcher');
    applyStaticLabels();
  }

  global.I18n = {
    t,
    setLocale,
    getLocale,
    getLocales,
    countryName,
    applyStaticLabels,
    renderLanguageSelector,
    init,
  };
})(window);
