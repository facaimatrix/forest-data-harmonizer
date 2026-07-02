// ── Tauri IPC wrappers ────────────────────────────────────────────────────────
let invokeApi = null;
let dialogApi = null;

async function initTauri() {
  try {
    if (typeof window.__TAURI__ !== 'undefined') {
      invokeApi = window.__TAURI__.core;
      dialogApi = window.__TAURI__.dialog;
    }
  } catch (e) { console.error('Tauri init failed:', e); }
}
const invoke = async (cmd, args = {}) => {
  if (!invokeApi) await initTauri();
  if (!invokeApi || typeof invokeApi.invoke !== 'function')
    throw new Error('Tauri core.invoke not available.');
  return invokeApi.invoke(cmd, args);
};
const openDialog = async (opts = {}) => {
  if (!dialogApi) await initTauri();
  if (!dialogApi || typeof dialogApi.open !== 'function') throw new Error('Dialog API not available');
  return dialogApi.open(opts);
};

// ── i18n ──────────────────────────────────────────────────────────────────────
const t = (key, params) => I18n.t(key, params);

// ── Constants ─────────────────────────────────────────────────────────────────
const HARMONIZE_STEP_KEYS = ['steps.contact', 'steps.mode', 'steps.load', 'steps.format', 'steps.inventory', 'steps.status', 'steps.validate', 'steps.export'];
const DIAGNOSE_STEP_KEYS  = ['steps.contact', 'steps.mode', 'steps.load', 'steps.diagnose'];

// Harmonize pipeline: Contact → Mode → Load → Format → …
const STEP = {
  CONTACT: 0,
  MODE: 1,
  LOAD: 2,
  FORMAT: 3,
  INVENTORY: 4,
  STATUS: 5,
  VALIDATE: 6,
  EXPORT: 7,
};

function harmonizeRender() {
  return [renderStep3, renderStep1, renderStep0, renderStep2, renderStep4, renderStep5, renderStep6, renderStep7][state.step] || renderStep3;
}

function diagnoseRender() {
  return [renderStep3, renderStep1, renderStep0, renderDiagnoseStep2][state.step] || renderStep3;
}

// ISO 3166-1 country list (name → iso3) — ordered alphabetically by name
const COUNTRIES = [
  {n:'Afghanistan',iso:'AFG'},{n:'Albania',iso:'ALB'},{n:'Algeria',iso:'DZA'},
  {n:'Angola',iso:'AGO'},{n:'Argentina',iso:'ARG'},{n:'Armenia',iso:'ARM'},
  {n:'Australia',iso:'AUS'},{n:'Austria',iso:'AUT'},{n:'Azerbaijan',iso:'AZE'},
  {n:'Bangladesh',iso:'BGD'},{n:'Belarus',iso:'BLR'},{n:'Belgium',iso:'BEL'},
  {n:'Belize',iso:'BLZ'},{n:'Benin',iso:'BEN'},{n:'Bhutan',iso:'BTN'},
  {n:'Bolivia',iso:'BOL'},{n:'Bosnia and Herzegovina',iso:'BIH'},
  {n:'Botswana',iso:'BWA'},{n:'Brazil',iso:'BRA'},{n:'Brunei',iso:'BRN'},
  {n:'Bulgaria',iso:'BGR'},{n:'Burkina Faso',iso:'BFA'},{n:'Burundi',iso:'BDI'},
  {n:'Cabo Verde',iso:'CPV'},{n:'Cambodia',iso:'KHM'},{n:'Cameroon',iso:'CMR'},
  {n:'Central African Republic',iso:'CAF'},{n:'Chad',iso:'TCD'},{n:'Chile',iso:'CHL'},
  {n:'China',iso:'CHN'},{n:'Colombia',iso:'COL'},{n:'Comoros',iso:'COM'},
  {n:'Congo (Brazzaville)',iso:'COG'},{n:'Congo (Kinshasa / DRC)',iso:'COD'},
  {n:'Costa Rica',iso:'CRI'},{n:"Côte d'Ivoire",iso:'CIV'},{n:'Croatia',iso:'HRV'},
  {n:'Cuba',iso:'CUB'},{n:'Czech Republic',iso:'CZE'},{n:'Denmark',iso:'DNK'},
  {n:'Djibouti',iso:'DJI'},{n:'Dominican Republic',iso:'DOM'},{n:'Ecuador',iso:'ECU'},
  {n:'Egypt',iso:'EGY'},{n:'El Salvador',iso:'SLV'},{n:'Equatorial Guinea',iso:'GNQ'},
  {n:'Eritrea',iso:'ERI'},{n:'Estonia',iso:'EST'},{n:'Eswatini',iso:'SWZ'},
  {n:'Ethiopia',iso:'ETH'},{n:'Fiji',iso:'FJI'},{n:'Finland',iso:'FIN'},
  {n:'France',iso:'FRA'},{n:'French Guiana',iso:'GUF'},{n:'Gabon',iso:'GAB'},
  {n:'Gambia',iso:'GMB'},{n:'Georgia',iso:'GEO'},{n:'Germany',iso:'DEU'},
  {n:'Ghana',iso:'GHA'},{n:'Greece',iso:'GRC'},{n:'Guatemala',iso:'GTM'},
  {n:'Guinea',iso:'GIN'},{n:'Guinea-Bissau',iso:'GNB'},{n:'Guyana',iso:'GUY'},
  {n:'Haiti',iso:'HTI'},{n:'Honduras',iso:'HND'},{n:'Hungary',iso:'HUN'},
  {n:'India',iso:'IND'},{n:'Indonesia',iso:'IDN'},{n:'Iran',iso:'IRN'},
  {n:'Iraq',iso:'IRQ'},{n:'Ireland',iso:'IRL'},{n:'Israel',iso:'ISR'},
  {n:'Italy',iso:'ITA'},{n:'Jamaica',iso:'JAM'},{n:'Japan',iso:'JPN'},
  {n:'Jordan',iso:'JOR'},{n:'Kazakhstan',iso:'KAZ'},{n:'Kenya',iso:'KEN'},
  {n:'Kyrgyzstan',iso:'KGZ'},{n:'Laos',iso:'LAO'},{n:'Latvia',iso:'LVA'},
  {n:'Lebanon',iso:'LBN'},{n:'Lesotho',iso:'LSO'},{n:'Liberia',iso:'LBR'},
  {n:'Libya',iso:'LBY'},{n:'Lithuania',iso:'LTU'},{n:'Luxembourg',iso:'LUX'},
  {n:'Madagascar',iso:'MDG'},{n:'Malawi',iso:'MWI'},{n:'Malaysia',iso:'MYS'},
  {n:'Mali',iso:'MLI'},{n:'Mauritania',iso:'MRT'},{n:'Mauritius',iso:'MUS'},
  {n:'Mexico',iso:'MEX'},{n:'Moldova',iso:'MDA'},{n:'Mongolia',iso:'MNG'},
  {n:'Morocco',iso:'MAR'},{n:'Mozambique',iso:'MOZ'},{n:'Myanmar',iso:'MMR'},
  {n:'Namibia',iso:'NAM'},{n:'Nepal',iso:'NPL'},{n:'Netherlands',iso:'NLD'},
  {n:'New Zealand',iso:'NZL'},{n:'Nicaragua',iso:'NIC'},{n:'Niger',iso:'NER'},
  {n:'Nigeria',iso:'NGA'},{n:'Norway',iso:'NOR'},{n:'Pakistan',iso:'PAK'},
  {n:'Panama',iso:'PAN'},{n:'Papua New Guinea',iso:'PNG'},{n:'Paraguay',iso:'PRY'},
  {n:'Peru',iso:'PER'},{n:'Philippines',iso:'PHL'},{n:'Poland',iso:'POL'},
  {n:'Portugal',iso:'PRT'},{n:'Romania',iso:'ROU'},{n:'Russia',iso:'RUS'},
  {n:'Rwanda',iso:'RWA'},{n:'São Tomé and Príncipe',iso:'STP'},
  {n:'Senegal',iso:'SEN'},{n:'Sierra Leone',iso:'SLE'},{n:'Singapore',iso:'SGP'},
  {n:'Solomon Islands',iso:'SLB'},{n:'Somalia',iso:'SOM'},{n:'South Africa',iso:'ZAF'},
  {n:'South Sudan',iso:'SSD'},{n:'Spain',iso:'ESP'},{n:'Sri Lanka',iso:'LKA'},
  {n:'Sudan',iso:'SDN'},{n:'Suriname',iso:'SUR'},{n:'Sweden',iso:'SWE'},
  {n:'Switzerland',iso:'CHE'},{n:'Taiwan',iso:'TWN'},{n:'Tajikistan',iso:'TJK'},
  {n:'Tanzania',iso:'TZA'},{n:'Thailand',iso:'THA'},{n:'Timor-Leste',iso:'TLS'},
  {n:'Togo',iso:'TGO'},{n:'Trinidad and Tobago',iso:'TTO'},{n:'Tunisia',iso:'TUN'},
  {n:'Turkey',iso:'TUR'},{n:'Turkmenistan',iso:'TKM'},{n:'Uganda',iso:'UGA'},
  {n:'Ukraine',iso:'UKR'},{n:'United Kingdom',iso:'GBR'},
  {n:'United States',iso:'USA'},{n:'Uruguay',iso:'URY'},{n:'Uzbekistan',iso:'UZB'},
  {n:'Venezuela',iso:'VEN'},{n:'Vietnam',iso:'VNM'},{n:'Yemen',iso:'YEM'},
  {n:'Zambia',iso:'ZMB'},{n:'Zimbabwe',iso:'ZWE'},
];

// GFB3 field definitions used in the Column Inventory step
const GFB3_FIELDS_INFO = [
  { role: 'plot_id',   label: 'PlotID',    type: 'text',    multi: true,  required: true,  desc: 'Unique identifier for each plot. All trees in the same plot share this ID. Must be consistent across censuses. Can be built from multiple columns (e.g. site + plot number).' },
  { role: 'pa',        label: 'PA',        type: 'number',  multi: false, required: false, desc: 'Plot area in hectares (e.g. 1.0). Used for per-hectare stem-density calculations. Can be a constant if all plots are the same size.' },
  { role: 'latitude',  label: 'Latitude',  type: 'number',  multi: false, required: false, desc: 'Plot centroid latitude in decimal degrees (WGS 84). Negative = south. Can be a constant if all plots share a centroid.' },
  { role: 'longitude', label: 'Longitude', type: 'number',  multi: false, required: false, desc: 'Plot centroid longitude in decimal degrees (WGS 84). Negative = west.' },
  { role: 'tree_id',   label: 'TreeID',    type: 'text',    multi: true,  required: true,  desc: 'Persistent identifier linking the same physical tree across census years. Must be unique within a plot. Can be built from multiple columns (e.g. plot + tag number).' },
  { role: 'species',   label: 'Species',   type: 'text',    multi: false, required: false, desc: 'Species code or full name. Preferably a standardised code (genus_species or a local code). Can be a constant "Unknown" if not recorded.' },
  { role: 'dbh',       label: 'DBH',       type: 'number',  multi: false, required: true,  desc: 'Stem diameter at breast height (1.3 m). Will be stored in cm — specify the source unit below. Dead trees will have DBH set to null at export.' },
  { role: 'yr',        label: 'YR',        type: 'integer', multi: false, required: true,  desc: 'Census year as a 4-digit integer (e.g. 2010). Used to compute PrevYR (the lag between successive censuses for each tree).' },
  { role: 'status',    label: 'Status',    type: 'text',    multi: false, required: false, desc: 'Tree fate code. GFB3 uses: 0 = alive, 1 = dead, 2 = recruit, 9 = missing. Leave unassigned to have the app derive status automatically from the census structure.' },
];

const ROLE_LABELS = {
  ignore:    '— Ignore —',
  plot_id:   'PlotID',
  tree_id:   'TreeID',
  pa:        'PA (plot area)',
  latitude:  'Latitude',
  longitude: 'Longitude',
  species:   'Species',
  dbh:       'DBH',
  yr:        'Census Year (YR)',
  status:    'Status',
};

// ── App state ─────────────────────────────────────────────────────────────────
const state = {
  mode:       null,   // 'harmonize' | 'diagnose'
  step:       0,

  loadResult: null,
  filePath:   null,
  dataFormat: null,   // 'long' | 'wide'

  // Contact & dataset (step 3)
  contact: { firstName: '', middleName: '', lastName: '' },
  countryName: '',    // full name, ISO3 derived from COUNTRIES lookup
  country:     '',    // ISO3 (derived)
  submitYear:  new Date().getFullYear(),
  censusType:  'multi', // 'single' | 'multi'
  gfb3Dsn:   '',
  siteName:  '',
  piName:    '',
  dbhUnit:   'cm',
  censusYears: [],

  // Field assignments (step 4 — long format)
  fa: {
    plotId:  { cols: [], prefix: '' },
    treeId:  { cols: [] },
    pa:      { col: '', literal: '' },
    lat:     { col: '', literal: '' },
    lon:     { col: '', literal: '' },
    species: { col: '' },
    dbh:     { col: '' },
    yr:      { col: '' },
    status:  { col: '' },
  },

  // Plot metadata lookup (Lat, Lon, PA from a separate file)
  plotLookup: {
    enabled:      false,
    filePath:     '',
    columns:      [],     // columns in the loaded lookup file
    mainKeyCol:   '',     // column in the MAIN data used as the join key (raw/native plot ID)
    lookupKeyCol: '',     // column in the lookup file that matches mainKeyCol
    latCol:       '',     // column in lookup file for Latitude  ('' = not using)
    lonCol:       '',     // column in lookup file for Longitude
    paCol:        '',     // column in lookup file for PA
  },
  coordFormat: 'decimal',   // 'decimal' | 'dm' | 'dms'

  // Wide format step 4 state
  wideStep:    0,            // 0 = select DBH cols, 1 = assign years
  wideDbhCols: [],           // column names checked as DBH columns
  wideLatCol:  '',
  wideLonCol:  '',

  // Status step
  statusMode:   'derive', // 'derive' | 'column'
  disappearedTreatment: 'dead',
  deriveResult: null,

  // Wide format (kept from original)
  columnMappings: [],
  widePairs: [],
  applyResult: null,
  validationReport: null,
};

// ── Utilities ─────────────────────────────────────────────────────────────────
function esc(s) {
  if (s == null) return '';
  const d = document.createElement('div');
  d.textContent = String(s);
  return d.innerHTML;
}
const el   = (id, fn) => { const e = document.getElementById(id); if (e) fn(e); };
const qsa  = (sel, fn) => document.querySelectorAll(sel).forEach(fn);
const bind = (id, fn) => el(id, e => e.addEventListener('input',  ev => fn(ev.target.value)));
const bindS= (id, fn) => el(id, e => e.addEventListener('change', ev => fn(ev.target.value)));

function showLoading(msg) {
  document.getElementById('loading').classList.add('visible');
  document.getElementById('loading-msg').textContent = msg || t('common.working');
}
function hideLoading() { document.getElementById('loading').classList.remove('visible'); }
function showError(msg) { const b = document.getElementById('error-bar'); b.textContent = msg; b.classList.add('visible'); }
function clearError() { document.getElementById('error-bar').classList.remove('visible'); }

function cols() { return state.loadResult ? state.loadResult.columns : []; }

function sampleVals(col) {
  const lr = state.loadResult;
  if (!lr) return [];
  const i = lr.columns.indexOf(col);
  if (i < 0) return [];
  return lr.preview_rows.map(r => r[i]).filter(v => v != null).slice(0, 4);
}

function isoFromCountry(name) {
  if (!name) return '';
  const match = COUNTRIES.find(c => c.n.toLowerCase() === name.toLowerCase());
  return match ? match.iso.toLowerCase() : '';
}

function computeDsn() {
  const iso  = isoFromCountry(state.countryName);
  const yr   = String(state.submitYear || '').replace(/\D/g, '').slice(0, 4);
  const last = (state.contact.lastName || '').toLowerCase().replace(/[\s-]+/g, '');
  const ct   = state.censusType === 'single' ? 's' : 'm';
  if (!iso || !yr || !last) return '';
  return `in_${iso}_${last}_${yr}_${ct}`;
}

function stepNames() {
  const keys = state.mode === 'diagnose' ? DIAGNOSE_STEP_KEYS : HARMONIZE_STEP_KEYS;
  return keys.map(k => t(k));
}

// ── Step indicator ─────────────────────────────────────────────────────────────
function renderStepIndicator() {
  return stepNames().map((name, i) => {
    const cls = i < state.step ? 'done' : i === state.step ? 'active' : '';
    const sep = i < stepNames().length - 1 ? '<span class="step-sep">›</span>' : '';
    return `<div class="step-pip ${cls}"><div class="num">${i}</div>${esc(name)}</div>${sep}`;
  }).join('');
}

// ── Step 0: Load ───────────────────────────────────────────────────────────────
function renderStep0() {
  const lr = state.loadResult;
  const gate = lr && lr.gate_errors.length
    ? `<div class="gate-errors"><h3>${t('step0.gateTitle')}</h3><ul>${lr.gate_errors.map(e=>`<li>${esc(e)}</li>`).join('')}</ul></div>`
    : (lr ? `<p class="file-chosen" style="margin-top:.5rem">${t('step0.structureOk')}</p>` : '');

  const preview = lr ? `
    <div class="preview-wrap">
      <div class="preview-meta">${t('step0.previewMeta', { rows: lr.row_count.toLocaleString(), cols: lr.columns.length, shown: lr.preview_rows.length })}</div>
      <div style="overflow-x:auto">
        <table><thead><tr>${lr.columns.map(c=>`<th>${esc(c)}</th>`).join('')}</tr></thead>
        <tbody>${lr.preview_rows.map(row=>`<tr>${row.map(v=>v==null?`<td class="null-cell">${t('common.null')}</td>`:`<td>${esc(v)}</td>`).join('')}</tr>`).join('')}</tbody></table>
      </div>
    </div>` : '';

  return `<div class="step-content">
    <h2>${t('step0.title')}</h2>
    <p class="step-desc">${t('step0.desc')}</p>
    <button class="btn btn-primary btn-lg" id="pick-file">${t('step0.pick')}</button>
    ${state.filePath ? `<p class="file-chosen" style="margin-top:.75rem;max-width:600px">${esc(state.filePath)}</p>` : ''}
    ${gate}${preview}
  </div>`;
}

// ── Step 1: Mode ───────────────────────────────────────────────────────────────
function renderStep1() {
  return `<div class="step-content">
    <h2>${t('step1.title')}</h2>
    <p class="step-desc">${t('step1.desc')}</p>
    <div class="mode-grid">
      <div class="mode-card ${state.mode==='harmonize'?'selected':''}" id="mode-harmonize">
        <div class="mode-icon">🔧</div>
        <h3>${t('step1.harmonize.title')}</h3>
        <p>${t('step1.harmonize.desc')}</p>
      </div>
      <div class="mode-card ${state.mode==='diagnose'?'selected':''}" id="mode-diagnose">
        <div class="mode-icon">🔍</div>
        <h3>${t('step1.diagnose.title')}</h3>
        <p>${t('step1.diagnose.desc')}</p>
      </div>
    </div>
  </div>`;
}

// ── Step 2: Format ─────────────────────────────────────────────────────────────
function renderStep2() {
  return `<div class="step-content">
    <h2>${t('step2.title')}</h2>
    <p class="step-desc">${t('step2.desc')}</p>
    <div class="format-choice-grid">
      <div class="format-choice-card ${state.dataFormat==='long'?'selected':''}" id="fmt-long">
        <h3>${t('step2.long.title')}</h3>
        <p>${t('step2.long.desc')}</p>
        <pre>PlotID | TreeID | YR   | DBH
  P1   |   T1   | 1994 | 12.3
  P1   |   T1   | 1997 | 13.1</pre>
      </div>
      <div class="format-choice-card ${state.dataFormat==='wide'?'selected':''}" id="fmt-wide">
        <h3>${t('step2.wide.title')}</h3>
        <p>${t('step2.wide.desc')}</p>
        <pre>PlotID | TreeID | DBH_1994 | DBH_1997
  P1   |   T1   |   12.3   |   13.1</pre>
      </div>
    </div>
  </div>`;
}

// ── Step 3: Contact & Dataset ──────────────────────────────────────────────────
function renderStep3() {
  const c = state.contact;
  const dsn = computeDsn();
  const iso = isoFromCountry(state.countryName).toUpperCase();
  const countryOpts = COUNTRIES.map(ct =>
    `<option value="${esc(ct.n)}" ${state.countryName===ct.n?'selected':''}>${esc(I18n.countryName(ct.iso, ct.n))}</option>`
  ).join('');

  return `<div class="step-content">
    <h2>${t('step3.title')}</h2>
    <p class="step-desc">${t('step3.desc')}</p>

    <p class="section-heading" style="margin-top:0">${t('step3.personHeading')}</p>
    <div class="form-grid" style="max-width:560px">
      <label>${t('step3.firstName')} <span class="required-mark">*</span></label>
      <input type="text" id="f-firstname" value="${esc(c.firstName)}" placeholder="${t('step3.phFirst')}" />

      <label>${t('step3.middleName')}</label>
      <input type="text" id="f-midname" value="${esc(c.middleName)}" placeholder="${t('common.optional')}" />

      <label>${t('step3.lastName')} <span class="required-mark">*</span></label>
      <input type="text" id="f-lastname" value="${esc(c.lastName)}" placeholder="${t('step3.phLast')}" />
    </div>

    <p class="section-heading">${t('step3.provenanceHeading')}</p>
    <div class="form-grid" style="max-width:560px">
      <label>${t('step3.country')} <span class="required-mark">*</span></label>
      <div>
        <select id="f-country" style="min-width:260px;padding:.4rem .6rem;border:1px solid var(--border);border-radius:var(--radius);font-size:.875rem;font-family:inherit;background:var(--surface)">
          <option value="">${t('common.selectCountry')}</option>
          ${countryOpts}
        </select>
        ${iso ? `<span style="font-family:monospace;font-size:.82rem;color:var(--green-dark);margin-left:.6rem">${esc(iso)}</span>` : ''}
      </div>

      <label>${t('step3.submitYear')} <span class="required-mark">*</span></label>
      <input type="number" id="f-submityear" value="${esc(state.submitYear)}" min="2000" max="2100" style="width:90px" />

      <label>${t('step3.censusType')} <span class="required-mark">*</span></label>
      <div>
        <div class="radio-group">
          <label style="align-items:flex-start;gap:.4rem">
            <input type="radio" name="census-type" value="multi"  ${state.censusType==='multi' ?'checked':''} style="margin-top:.2rem" />
            <span><strong>${t('step3.multiCensus')}</strong> <span style="color:var(--text-muted);font-size:.82rem">${t('step3.multiCensusHint')}</span></span>
          </label>
        </div>
        <div class="radio-group" style="margin-top:.4rem">
          <label style="align-items:flex-start;gap:.4rem">
            <input type="radio" name="census-type" value="single" ${state.censusType==='single'?'checked':''} style="margin-top:.2rem" />
            <span><strong>${t('step3.singleCensus')}</strong> <span style="color:var(--text-muted);font-size:.82rem">${t('step3.singleCensusHint')}</span></span>
          </label>
        </div>
      </div>

      <label>${t('step3.siteName')}</label>
      <input type="text" id="f-site" value="${esc(state.siteName)}" placeholder="${t('step3.phSite')}" />

      <label>${t('step3.pi')}</label>
      <input type="text" id="f-pi" value="${esc(state.piName)}" placeholder="${t('step3.phPi')}" />

      <label>${t('step3.dbhUnit')}</label>
      <div class="radio-group">
        <label><input type="radio" name="dbh-unit" value="cm" ${state.dbhUnit==='cm'?'checked':''} /> cm</label>
        <label><input type="radio" name="dbh-unit" value="mm" ${state.dbhUnit==='mm'?'checked':''} /> mm <span style="color:var(--text-muted)">${t('step3.mmHint')}</span></label>
      </div>

    </div>

    <div style="margin-top:1.25rem;padding:.9rem 1.1rem;background:var(--green-pale);border:1px solid var(--green-light);border-radius:var(--radius);max-width:720px">
      <div style="font-size:.78rem;color:var(--green-dark);font-weight:600;margin-bottom:.3rem">${t('step3.dsnLabel')}</div>
      <div style="font-family:monospace;font-size:1.05rem;color:var(--green-dark)" id="dsn-preview">
        ${dsn ? esc(dsn) : `<span style="color:var(--text-muted);font-style:italic">${t('step3.dsnEmpty')}</span>`}
      </div>
      <div style="font-size:.75rem;color:var(--text-muted);margin-top:.3rem">${t('step3.dsnFormat')}</div>
    </div>
  </div>`;
}

// ── Step 4 helpers ────────────────────────────────────────────────────────────
function colSelect(id, value, extra='') {
  const opts = `<option value="">${t('common.none')}</option>` +
    cols().map(c => `<option value="${esc(c)}" ${value===c?'selected':''}>${esc(c)}</option>`).join('');
  return `<select class="fa-sel" id="${id}" style="min-width:200px;padding:.35rem .5rem;border:1px solid var(--border);border-radius:var(--radius);font-size:.83rem;font-family:inherit;background:var(--surface)" ${extra}>${opts}</select>`;
}

function multiColPills(faKey) {
  const arr = state.fa[faKey].cols;
  const available = cols().filter(c => !arr.includes(c));
  const pills = arr.map((c,i) =>
    `<span class="fa-pill" data-fakey="${faKey}" data-idx="${i}">${esc(c)}<button class="fa-pill-remove" data-fakey="${faKey}" data-idx="${i}" title="${t('common.remove')}">×</button></span>`
  ).join('');
  const addSel = available.length
    ? `<select class="fa-add-sel" data-fakey="${faKey}" style="padding:.3rem .5rem;border:1px solid var(--border);border-radius:var(--radius);font-size:.8rem;font-family:inherit;background:var(--surface);color:var(--text-muted)">
        <option value="">${t('common.addColumn')}</option>
        ${available.map(c=>`<option value="${esc(c)}">${esc(c)}</option>`).join('')}
       </select>`
    : '';
  return `<div class="fa-pills" style="display:flex;align-items:center;gap:.35rem;flex-wrap:wrap">${pills}${addSel}</div>`;
}

function concatPreview(cols_, prefix='') {
  const parts = [...(prefix?[prefix.toLowerCase()]:[]), ...cols_.map(c=>c.toLowerCase())];
  return parts.length ? parts.join('_') : '…';
}

function sampleForCol(c) {
  const vals = sampleVals(c);
  return vals.length ? `<span class="fa-sample">${vals.slice(0,3).map(esc).join(', ')}</span>` : '';
}

// ── Step 4: Field Assignment ───────────────────────────────────────────────────
function renderPlotMetaSection() {
  const fa = state.fa;
  const pl = state.plotLookup;
  const allCols = cols();
  const lookupCols = pl.columns;

  const lookupColSel = (id, val, placeholder) => {
    const ph = placeholder || t('common.none');
    const opts = `<option value="">${ph}</option>` +
      lookupCols.map(c => `<option value="${esc(c)}" ${val===c?'selected':''}>${esc(c)}</option>`).join('');
    return `<select id="${id}" style="min-width:160px;padding:.3rem .5rem;border:1px solid var(--border);border-radius:var(--radius);font-size:.82rem;font-family:inherit;background:var(--surface)">${opts}</select>`;
  };

  const mainColSel = (id, val) => {
    const opts = `<option value="">${t('common.selectColumn')}</option>` +
      allCols.map(c => `<option value="${esc(c)}" ${val===c?'selected':''}>${esc(c)}</option>`).join('');
    return `<select id="${id}" style="min-width:200px;padding:.3rem .5rem;border:1px solid var(--border);border-radius:var(--radius);font-size:.82rem;font-family:inherit;background:var(--surface)">${opts}</select>`;
  };

  const lookupConfig = pl.enabled ? `
    <div style="margin-top:.75rem;padding:.75rem 1rem;background:var(--surface-alt,#f9f9f9);border:1px solid var(--border);border-radius:var(--radius)">
      <div class="form-grid" style="max-width:560px;row-gap:.55rem">
        <label class="fa-sublabel">${t('fields.lookupFile')}</label>
        <div style="display:flex;align-items:center;gap:.5rem">
          <input type="text" id="lookup-path-display" value="${esc(pl.filePath)}" readonly
            style="flex:1;font-size:.82rem;padding:.3rem .5rem;border:1px solid var(--border);border-radius:var(--radius);background:var(--surface)" />
          <button class="btn btn-ghost" id="pick-lookup-file" style="font-size:.8rem;padding:.3rem .65rem">${t('common.browse')}</button>
        </div>

        ${pl.filePath ? `
        <label class="fa-sublabel">${t('fields.nativePlotId')}</label>
        ${mainColSel('lookup-main-key', pl.mainKeyCol)}

        <label class="fa-sublabel">${t('fields.joinKey')}</label>
        ${lookupColSel('lookup-lk-key', pl.lookupKeyCol, t('common.select'))}

        <label class="fa-sublabel">${t('fields.latCol')}</label>
        ${lookupColSel('lookup-lat-col', pl.latCol)}

        <label class="fa-sublabel">${t('fields.lonCol')}</label>
        ${lookupColSel('lookup-lon-col', pl.lonCol)}

        <label class="fa-sublabel">${t('fields.paCol')}</label>
        ${lookupColSel('lookup-pa-col', pl.paCol)}
        ` : `<p style="font-size:.82rem;color:var(--text-muted);margin:.25rem 0 0">${t('fields.pickFileHint')}</p>`}
      </div>
    </div>` : '';

  // Only show direct col/literal inputs for fields not supplied by the lookup
  const showLat = !pl.enabled || !pl.latCol;
  const showLon = !pl.enabled || !pl.lonCol;
  const showPa  = !pl.enabled || !pl.paCol;

  const directInputs = (showLat || showLon || showPa) ? `
    <div class="fa-grid" style="margin-top:.6rem">
      ${showLat ? `<div class="fa-row">
        <div class="fa-left">
          <div class="fa-label">${t('fields.latitude')}</div>
          <div class="fa-type">${t('fields.decimalDegrees')}</div>
          <div class="fa-desc">${t('fields.latDesc')}</div>
        </div>
        <div class="fa-right">
          <div style="display:flex;align-items:center;gap:.5rem;flex-wrap:wrap">
            ${colSelect('fa-lat', fa.lat.col)}
            <span style="color:var(--text-muted);font-size:.78rem">${t('common.orConstant')}</span>
            <input type="number" step="any" id="fa-lat-lit" class="fa-text-input" value="${esc(fa.lat.literal)}"
              placeholder="${t('fields.phLat')}" style="width:120px" ${fa.lat.col?'disabled':''} />
            <button class="btn btn-ghost" id="open-map-btn" style="font-size:.78rem;padding:.25rem .55rem">${t('fields.mapBtn')}</button>
          </div>
          ${fa.lat.col ? `
            <div style="margin-top:.4rem">
              <label class="fa-sublabel">${t('fields.coordFormat')}</label>
              <select id="coord-format-sel" class="fa-text-input" style="width:auto;margin-top:.2rem">
                <option value="decimal" ${state.coordFormat==='decimal'?'selected':''}>${t('fields.coordDecimal')}</option>
                <option value="dm"      ${state.coordFormat==='dm'     ?'selected':''}>${t('fields.coordDm')}</option>
                <option value="dms"     ${state.coordFormat==='dms'    ?'selected':''}>${t('fields.coordDms')}</option>
              </select>
            </div>` : ''}
        </div>
      </div>` : ''}
      ${showLon ? `<div class="fa-row">
        <div class="fa-left">
          <div class="fa-label">${t('fields.longitude')}</div>
          <div class="fa-type">${t('fields.decimalDegrees')}</div>
          <div class="fa-desc">${t('fields.lonDesc')}</div>
        </div>
        <div class="fa-right">
          <div style="display:flex;align-items:center;gap:.5rem;flex-wrap:wrap">
            ${colSelect('fa-lon', fa.lon.col)}
            <span style="color:var(--text-muted);font-size:.78rem">${t('common.orConstant')}</span>
            <input type="number" step="any" id="fa-lon-lit" class="fa-text-input" value="${esc(fa.lon.literal)}"
              placeholder="${t('fields.phLon')}" style="width:120px" ${fa.lon.col?'disabled':''} />
          </div>
        </div>
      </div>` : ''}
      ${showPa ? `<div class="fa-row">
        <div class="fa-left">
          <div class="fa-label">${t('fields.pa')}</div>
          <div class="fa-type">${t('fields.numberHa')}</div>
          <div class="fa-desc">${t('fields.paDesc')}</div>
        </div>
        <div class="fa-right">
          <div style="display:flex;align-items:center;gap:.5rem;flex-wrap:wrap">
            ${colSelect('fa-pa', fa.pa.col)}
            <span style="color:var(--text-muted);font-size:.78rem">${t('common.orConstant')}</span>
            <input type="number" step="any" id="fa-pa-lit" class="fa-text-input" value="${esc(fa.pa.literal)}"
              placeholder="${t('fields.phPa')}" style="width:90px" ${fa.pa.col?'disabled':''} />
            <span style="font-size:.78rem;color:var(--text-muted)">${t('common.ha')}</span>
          </div>
        </div>
      </div>` : ''}
    </div>` : '';

  return `<div style="margin-bottom:1.25rem;padding:.85rem 1rem;border:1px solid var(--border);border-radius:var(--radius);background:var(--surface)">
    <p class="section-heading" style="margin:0 0 .5rem">${t('fields.plotMetaHeading')}</p>
    <label style="display:flex;align-items:center;gap:.5rem;cursor:pointer;font-size:.85rem">
      <input type="checkbox" id="lookup-enabled" ${pl.enabled?'checked':''} style="accent-color:var(--green)" />
      ${t('fields.lookupEnable')}
    </label>
    <p style="font-size:.78rem;color:var(--text-muted);margin:.2rem 0 0 1.5rem">${t('fields.lookupHint')}</p>
    ${lookupConfig}
    ${directInputs}
  </div>`;
}

function renderStep4() {
  if (state.dataFormat === 'wide') return renderStep4Wide();

  const allCols = cols();
  if (!allCols.length) return `<div class="step-content"><h2>${t('step4.title')}</h2><p>${t('step4.noColumns')}</p></div>`;

  const fa = state.fa;
  const plotPreview = concatPreview(fa.plotId.cols, fa.plotId.prefix);
  const treePreview = concatPreview(fa.treeId.cols);

  const rows = [
    // ── PlotID ──
    `<div class="fa-row">
      <div class="fa-left">
        <div class="fa-label">${t('step4.plotId')} <span class="fa-req">✱</span></div>
        <div class="fa-type">${t('step4.text')}</div>
        <div class="fa-desc">${t('step4.plotIdDesc')}</div>
        <div class="fa-tip">${t('step4.plotIdTip')}</div>
      </div>
      <div class="fa-right">
        ${multiColPills('plotId')}
        <div style="margin-top:.6rem">
          <label class="fa-sublabel">${t('step4.sitePrefix')}</label>
          <div style="display:flex;align-items:center;gap:.35rem;margin-top:.2rem">
            <input type="text" id="fa-plotid-prefix" class="fa-text-input" value="${esc(fa.plotId.prefix)}" placeholder="${t('step4.phPrefix')}" style="width:140px" />
            <span style="color:var(--text-muted);font-size:.78rem">${t('step4.prefixSuffix')}</span>
          </div>
        </div>
        ${(fa.plotId.cols.length || fa.plotId.prefix) ? `<div class="fa-preview" id="plotid-preview">→ <strong>${esc(plotPreview)}</strong></div>` : `<div class="fa-preview" id="plotid-preview" style="display:none"></div>`}
      </div>
    </div>`,

    // ── TreeID ──
    `<div class="fa-row">
      <div class="fa-left">
        <div class="fa-label">${t('step4.treeId')} <span class="fa-req">✱</span></div>
        <div class="fa-type">${t('step4.text')}</div>
        <div class="fa-desc">${t('step4.treeIdDesc')}</div>
        <div class="fa-tip">${t('step4.treeIdTip')}</div>
      </div>
      <div class="fa-right">
        ${multiColPills('treeId')}
        ${fa.treeId.cols.length ? `<div class="fa-preview">→ <strong>${esc(treePreview)}</strong></div>` : ''}
      </div>
    </div>`,

    // ── DBH ──
    `<div class="fa-row">
      <div class="fa-left">
        <div class="fa-label">${t('step4.dbh')} <span class="fa-req">✱</span></div>
        <div class="fa-type">${t('step4.number')} (${state.dbhUnit})</div>
        <div class="fa-desc">${t('step4.dbhDesc', { unit: state.dbhUnit })}</div>
      </div>
      <div class="fa-right">
        ${colSelect('fa-dbh', fa.dbh.col)}
        ${fa.dbh.col ? sampleForCol(fa.dbh.col) : ''}
      </div>
    </div>`,

    // ── YR ──
    `<div class="fa-row">
      <div class="fa-left">
        <div class="fa-label">${t('step4.yr')} <span class="fa-req">✱</span></div>
        <div class="fa-type">${t('step4.integer')}</div>
        <div class="fa-desc">${t('step4.yrDesc')}</div>
      </div>
      <div class="fa-right">
        ${colSelect('fa-yr', fa.yr.col)}
        ${fa.yr.col ? sampleForCol(fa.yr.col) : ''}
      </div>
    </div>`,

    // ── Species ──
    `<div class="fa-row">
      <div class="fa-left">
        <div class="fa-label">${t('step4.species')}</div>
        <div class="fa-type">${t('step4.text')}</div>
        <div class="fa-desc">${t('step4.speciesDesc')}</div>
      </div>
      <div class="fa-right">
        ${colSelect('fa-species', fa.species.col)}
        ${fa.species.col ? sampleForCol(fa.species.col) : ''}
      </div>
    </div>`,

    // ── Status ──
    `<div class="fa-row">
      <div class="fa-left">
        <div class="fa-label">${t('step4.status')}</div>
        <div class="fa-type">${t('step4.text')}</div>
        <div class="fa-desc">${t('step4.statusDesc')}</div>
      </div>
      <div class="fa-right">
        ${colSelect('fa-status', fa.status.col)}
        ${fa.status.col ? sampleForCol(fa.status.col) : `<div style="font-size:.78rem;color:var(--text-muted);margin-top:.3rem">${t('step4.statusBlank')}</div>`}
      </div>
    </div>`,
  ];

  return `<div class="step-content">
    <h2>${t('step4.title')}</h2>
    <p class="step-desc">${t('step4.desc')}</p>
    ${renderPlotMetaSection()}
    <div class="fa-grid">${rows.join('')}</div>
  </div>`;
}

// ── Step 4 Wide ───────────────────────────────────────────────────────────────
function renderStep4Wide() {
  return state.wideStep === 0 ? renderStep4WideSub0() : renderStep4WideSub1();
}

function renderStep4WideSub0() {
  const allCols = cols();
  const fa = state.fa;

  const checkboxes = allCols.map(col => {
    const checked = state.wideDbhCols.includes(col);
    const samples = sampleVals(col);
    const sv = samples.length ? ` <span class="fa-sample">${samples.slice(0,3).map(esc).join(', ')}</span>` : '';
    return `<label class="dbh-col-label ${checked?'checked':''}">
      <input type="checkbox" class="wide-dbh-chk" data-col="${esc(col)}" ${checked?'checked':''} />
      <span style="font-family:monospace;font-size:.83rem">${esc(col)}</span>${sv}
    </label>`;
  }).join('');

  const idFields = buildWideIdRows(allCols);

  return `<div class="step-content">
    <h2>${t('wide.step1.title')}</h2>
    <p class="step-desc">${t('wide.step1.desc')}</p>

    <p class="section-heading" style="margin-top:0">${t('wide.dbhHeading')} <span class="fa-req">✱</span></p>
    <div class="dbh-checkbox-grid">${checkboxes}</div>
    ${state.wideDbhCols.length
      ? `<p style="font-size:.82rem;color:var(--green-mid);margin-top:.5rem">${t('wide.selected', { count: state.wideDbhCols.length })}</p>`
      : `<p class="mapping-required">${t('wide.selectOne')}</p>`}

    ${renderPlotMetaSection()}
    <p class="section-heading" style="margin-top:1.25rem">${t('wide.identityHeading')}</p>
    <div class="fa-grid" style="margin-top:0">${idFields}</div>
  </div>`;
}

function buildWideIdRows(allCols) {
  const fa = state.fa;
  return [
    fieldRowWide(t('step4.plotId'), 'plotId', true),
    fieldRowWide(t('step4.treeId'), 'treeId', true),
    singleRowWide(t('step4.species'), 'fa-wide-species', fa.species.col, allCols, 'species'),
    singleRowWide(t('step4.status'), 'fa-wide-status', fa.status.col, allCols, 'status'),
  ].join('');
}

function fieldRowWide(label, faKey, required) {
  const fa = state.fa;
  const isTree = faKey === 'treeId';
  const tip = isTree ? t('wide.treeTip') : t('wide.plotTip');
  const preview = concatPreview(fa[faKey].cols, faKey === 'plotId' ? fa[faKey].prefix : '');
  return `<div class="fa-row">
    <div class="fa-left">
      <div class="fa-label">${label} ${required?'<span class="fa-req">✱</span>':''}</div>
      <div class="fa-desc">${tip}</div>
    </div>
    <div class="fa-right">
      ${multiColPills(faKey)}
      ${faKey === 'plotId' ? `
        <div style="margin-top:.5rem;display:flex;align-items:center;gap:.35rem">
          <label class="fa-sublabel">${t('wide.sitePrefix')}</label>
          <input type="text" id="fa-plotid-prefix" class="fa-text-input" value="${esc(fa.plotId.prefix)}" placeholder="${t('step4.phPrefix')}" style="width:120px" />
        </div>` : ''}
      ${(fa[faKey].cols.length || (faKey === 'plotId' && fa.plotId.prefix)) ? `<div class="fa-preview">→ <strong>${esc(preview)}</strong></div>` : ''}
    </div>
  </div>`;
}

function singleRowWide(label, id, value, allCols, fieldKey) {
  const opts = `<option value="">${t('common.none')}</option>` +
    allCols.map(c=>`<option value="${esc(c)}" ${value===c?'selected':''}>${esc(c)}</option>`).join('');
  return `<div class="fa-row">
    <div class="fa-left"><div class="fa-label">${label}</div></div>
    <div class="fa-right">
      <select class="fa-sel fa-wide-single" id="${id}" data-widefield="${fieldKey}" style="min-width:200px;padding:.35rem .5rem;border:1px solid var(--border);border-radius:var(--radius);font-size:.83rem;font-family:inherit;background:var(--surface)">${opts}</select>
    </div>
  </div>`;
}

function renderStep4WideSub1() {
  if (!state.wideDbhCols.length) return `<div class="step-content"><h2>${t('wide.step2.title')}</h2><p>${t('wide.step2.backHint')}</p></div>`;

  const rows = state.wideDbhCols.map(col => {
    const ex = state.widePairs.find(p => p.source_column === col);
    const samples = sampleVals(col);
    const sv = samples.length ? `<span class="fa-sample">${samples.slice(0,3).map(esc).join(', ')}</span>` : '';
    return `<div class="fa-row">
      <div class="fa-left">
        <div class="fa-label" style="font-family:monospace;font-size:.88rem">${esc(col)}</div>
        ${sv}
      </div>
      <div class="fa-right">
        <div style="display:flex;align-items:center;gap:.5rem">
          <label class="fa-sublabel">${t('wide.censusYear')}</label>
          <input type="number" class="year-input fa-text-input" data-col="${esc(col)}" value="${ex?ex.year:''}" min="1800" max="2100" placeholder="${t('wide.phYear')}" style="width:100px" />
        </div>
      </div>
    </div>`;
  }).join('');

  const validPairs = state.widePairs.filter(p => state.wideDbhCols.includes(p.source_column) && p.year > 0);
  const minCensuses = state.censusType === 'single' ? 1 : 2;
  const statusMsg = validPairs.length >= minCensuses
    ? `<p style="font-size:.82rem;color:var(--green-mid);margin:.5rem 0">${t('wide.yearsAssigned', { count: validPairs.length, years: validPairs.map(p=>p.year).sort((a,b)=>a-b).join(', ') })}</p>`
    : `<p class="mapping-required">${t('wide.assignAll', { min: minCensuses })}</p>`;

  return `<div class="step-content">
    <h2>${t('wide.step2.title')}</h2>
    <p class="step-desc">${t('wide.step2.desc')}</p>
    <div class="fa-grid">${rows}</div>
    ${statusMsg}
  </div>`;
}

// ── Step 5: Status ─────────────────────────────────────────────────────────────
function renderStep5() {
  const dr = state.deriveResult;
  const mode = state.statusMode;
  // Check if the user already assigned a Status column in the inventory
  const statusCol = cols().find(c => state.colRoles[c] === 'status');

  const tabs = `<div class="field-mode-tabs">
    <button class="fmode-btn ${mode==='derive'?'active':''}" data-field="status" data-mode="derive">${t('step5.tabDerive')}</button>
    <button class="fmode-btn ${mode==='column'?'active':''}" data-field="status" data-mode="column">${statusCol ? t('step5.tabColumnNamed', { col: statusCol }) : t('step5.tabColumn')}</button>
  </div>`;

  let body = '';
  if (mode === 'column') {
    const colOpts = `<option value="">${t('common.select')}</option>` + cols().map(c=>`<option value="${esc(c)}" ${(state.statusColOverride||statusCol)===c?'selected':''}>${esc(c)}</option>`).join('');
    body = `
      <p style="font-size:.83rem;color:var(--text-muted);margin-bottom:.75rem">${t('step5.columnHint')}</p>
      ${statusCol ? `<p style="font-size:.83rem;margin-bottom:.5rem">${t('step5.columnPre')} <strong><code>${esc(statusCol)}</code></strong></p>` : ''}
      <select class="field-col-select" id="fm-status-col">${colOpts}</select>`;
  } else {
    body = `
      <div class="info-box" style="margin-bottom:1rem">
        <strong>${t('step5.rulesTitle')}</strong><br>
        • ${t('step5.rule1')}<br>
        • ${t('step5.rule2')}<br>
        • ${t('step5.rule3')}<br>
        • ${t('step5.rule4')}
      </div>
      ${dr && dr.summary.disappeared_tree_count > 0 ? renderDisappearedBox() : ''}
      ${dr ? renderDeriveStats(dr) : `<p style="font-size:.83rem;color:var(--text-muted)">${t('step5.clickDerive')}</p>`}
      <button class="btn btn-primary" id="run-derive-btn" style="margin-top:1rem">${t('step5.deriveBtn')}</button>`;
  }

  return `<div class="step-content">
    <h2>${t('step5.title')}</h2>
    <p class="step-desc">${t('step5.desc')}</p>
    ${tabs}
    <div class="field-card" style="margin-top:.75rem">${body}</div>
  </div>`;
}

function renderDeriveStats(dr) {
  const s = dr.summary;
  const cards = [
    { n: s.first_census_count,    lbl: t('step5.stat.firstCensus'),         sub: t('step5.stat.firstCensusSub'),   cls: '' },
    { n: s.subsequent_alive_count,lbl: t('step5.stat.subsequent'),      sub: t('step5.stat.subsequentSub'),                    cls: '' },
    { n: s.recruit_count,         lbl: t('step5.stat.recruits'),                   sub: t('step5.stat.recruitsSub'),                    cls: '' },
    { n: s.dead_count,            lbl: t('step5.stat.dead'),  sub: t('step5.stat.deadSub'),            cls: s.dead_count    > 0 ? 'stat-warn' : '' },
    { n: s.missing_count,         lbl: t('step5.stat.missing'), sub: t('step5.stat.missingSub'),             cls: s.missing_count > 0 ? 'stat-warn' : '' },
    { n: s.disappeared_tree_count,lbl: t('step5.stat.disappeared'), sub: t('step5.stat.disappearedSub'), cls: s.disappeared_tree_count > 0 ? 'stat-warn' : '' },
  ];
  return `<div class="derive-stat-grid">${cards.map(c =>
    `<div class="derive-stat-card ${c.cls}">
      <div class="stat-num">${c.n.toLocaleString()}</div>
      <div class="stat-lbl">${c.lbl}</div>
      <div class="stat-sub">${c.sub}</div>
    </div>`).join('')}</div>`;
}

function renderDisappearedBox() {
  const treatment = state.disappearedTreatment;
  return `<div class="disappeared-box">
    <h4>${t('step5.disappeared.title')}</h4>
    <p>${t('step5.disappeared.desc')}</p>
    <div class="treatment-opts">
      <button class="treatment-btn ${treatment==='dead'   ?'selected':''}" data-treatment="dead">${t('step5.treat.dead')}</button>
      <button class="treatment-btn ${treatment==='missing'?'selected':''}" data-treatment="missing">${t('step5.treat.missing')}</button>
      <button class="treatment-btn ${treatment==='none'   ?'selected':''}" data-treatment="none">${t('step5.treat.none')}</button>
    </div>
  </div>`;
}

// ── Step 6: Validation ─────────────────────────────────────────────────────────
function renderStep6() {
  const report = state.validationReport;
  if (!report) return `<div class="step-content"><h2>${t('step6.title')}</h2><p class="step-desc">${t('step6.loading')}</p></div>`;
  const findings = report.findings;
  const bySev = sev => findings.filter(f=>f.severity===sev).length;
  const badges = [
    {sev:'AutoDrop',      label:t('sev.autoDrop'),    cls:'auto'},
    {sev:'AutoRecode',    label:t('sev.autoRecode'),  cls:'recode'},
    {sev:'RequiresInput', label:t('sev.needsReview'), cls:'input'},
    {sev:'Escalate',      label:t('sev.escalate'),     cls:'escalate'},
  ].filter(b=>bySev(b.sev)>0).map(b=>`<span class="badge badge-${b.cls}">${bySev(b.sev)} ${b.label}</span>`).join('');

  const findHtml = findings.length === 0
    ? `<div class="finding finding-clean"><div class="finding-header"><span class="finding-rule">${t('step6.allPassed')}</span><span class="badge badge-clean">${t('step6.clean')}</span></div><p class="finding-message">${t('step6.noIssues')}</p></div>`
    : findings.map(f => {
        const cls = {AutoDrop:'auto',AutoRecode:'recode',RequiresInput:'input',Escalate:'escalate'}[f.severity]||'recode';
        return `<div class="finding finding-${cls}">
          <div class="finding-header">
            <span class="finding-rule">${esc(fmtRule(f.rule))}</span>
            <span class="badge badge-${cls}">${esc(fmtSev(f.severity))}</span>
            <span class="finding-count">${t('step6.rowCount', { count: f.row_count.toLocaleString() })}</span>
          </div>
          <p class="finding-message">${esc(f.message)}</p>
          <div class="finding-action">${t('common.action')}: <strong>${esc(fmtAction(f.action))}</strong></div>
        </div>`;
      }).join('');

  const blockers = findings.some(f=>f.severity==='RequiresInput'||f.severity==='Escalate');
  return `<div class="step-content">
    <h2>${t('step6.title')}</h2>
    <p class="step-desc">${t('step6.desc')}</p>
    ${badges ? `<div class="findings-summary">${badges}</div>` : ''}
    ${findHtml}
    ${blockers ? `<div class="gate-errors" style="margin-top:1rem"><h3>${t('step6.reviewRequired')}</h3><p style="font-size:.85rem;color:var(--error)">${t('step6.reviewHint')}</p></div>` : ''}
  </div>`;
}

// ── Step 7: Export ─────────────────────────────────────────────────────────────
function renderStep7() {
  return `<div class="step-content">
    <h2>${t('step7.title')}</h2>
    <p class="step-desc">${t('step7.desc')}</p>
    <div class="form-grid" style="max-width:520px;margin-bottom:1rem">
      <label>${t('step7.outDir')}</label>
      <div style="display:flex;gap:.5rem;align-items:center">
        <input type="text" id="f-outdir" placeholder="${t('step7.phOutDir')}" style="flex:1" readonly />
        <button class="btn btn-ghost" id="pick-outdir">${t('common.browse')}</button>
      </div>
      <label>${t('step7.baseName')}</label>
      <input type="text" id="f-basename" value="${esc(state.gfb3Dsn || 'dataset')}" />
    </div>
    <div class="export-formats">
      <label class="format-card"><input type="checkbox" name="fmt" value="csv"     checked /> CSV</label>
      <label class="format-card"><input type="checkbox" name="fmt" value="parquet" checked /> Parquet</label>
      <label class="format-card"><input type="checkbox" name="fmt" value="xlsx"    checked /> XLSX</label>
    </div>
    <button class="btn btn-primary btn-lg" id="do-export">${t('step7.exportBtn')}</button>
    <div id="export-result"></div>
    <h3 style="margin:1.25rem 0 .4rem;font-size:.9rem;color:var(--text-muted)">${t('step7.logPreview')}</h3>
    <div class="curation-preview">${esc(buildCurationLog())}</div>
  </div>`;
}

// ── Diagnose mode step 2 ───────────────────────────────────────────────────────
function renderDiagnoseStep2() {
  const report = state.validationReport;
  if (!report) return `<div class="step-content"><h2>${t('diagnose.running')}</h2></div>`;
  const findings = report.findings;
  const bySev = sev => findings.filter(f=>f.severity===sev).length;
  const badges = ['AutoDrop','AutoRecode','RequiresInput','Escalate']
    .filter(s=>bySev(s)>0)
    .map(s=>{const cls={AutoDrop:'auto',AutoRecode:'recode',RequiresInput:'input',Escalate:'escalate'}[s]; return `<span class="badge badge-${cls}">${bySev(s)} ${fmtSev(s)}</span>`;}).join('');

  const findHtml = findings.length === 0
    ? `<div class="finding finding-clean"><div class="finding-header"><span class="finding-rule">${t('step6.allPassed')}</span><span class="badge badge-clean">${t('step6.clean')}</span></div><p class="finding-message">${t('diagnose.allPassedMsg')}</p></div>`
    : findings.map(f=>{const cls={AutoDrop:'auto',AutoRecode:'recode',RequiresInput:'input',Escalate:'escalate'}[f.severity]||'recode';
        return `<div class="finding finding-${cls}"><div class="finding-header"><span class="finding-rule">${esc(fmtRule(f.rule))}</span><span class="badge badge-${cls}">${esc(fmtSev(f.severity))}</span><span class="finding-count">${t('step6.rowCount', { count: f.row_count.toLocaleString() })}</span></div><p class="finding-message">${esc(f.message)}</p><div class="finding-action">${t('common.action')}: <strong>${esc(fmtAction(f.action))}</strong></div></div>`;
      }).join('');

  return `<div class="step-content">
    <h2>${t('diagnose.title')}</h2>
    <p style="font-size:.83rem;color:var(--text-muted);margin-bottom:1rem">
      ${t('diagnose.fileMeta', {
        name: esc(state.filePath?(state.filePath.split(/[/\\]/).pop()):''),
        rows: (state.loadResult?.row_count??0).toLocaleString(),
        cols: (state.loadResult?.columns??[]).length,
      })}
    </p>
    ${badges ? `<div class="findings-summary">${badges}</div>` : ''}
    ${findHtml}
  </div>`;
}

// ── Helpers ────────────────────────────────────────────────────────────────────
function fmtSev(s) {
  return {
    AutoDrop: t('sev.autoDrop'),
    AutoRecode: t('sev.autoRecode'),
    RequiresInput: t('sev.needsReview'),
    Escalate: t('sev.escalate'),
  }[s] || s;
}
function fmtRule(r) {
  return {
    DuplicateTreeWithinPlotYear: t('rule.duplicate'),
    UnknownStatus: t('rule.unknownStatus'),
    DeadTreeHasDbh: t('rule.deadHasDbh'),
    RecruitMissingDbh: t('rule.recruitMissingDbh'),
    OrphanDeadFirstCensus: t('rule.orphanDead'),
    RecruitAtMinYear: t('rule.recruitMinYear'),
    NoPersistentTreeId: t('rule.noPersistentTreeId'),
  }[r] || r;
}
function fmtAction(a) {
  return {
    DropRows: t('action.dropRows'),
    RecodeToMissing: t('action.recodeMissing'),
    NullifyDbh: t('action.nullifyDbh'),
    ContributorMapping: t('action.contributorMapping'),
    EscalateToCurationLog: t('action.escalateLog'),
    ReviewAndConfirm: t('action.reviewConfirm'),
  }[a] || a;
}

function buildCurationLog() {
  const lines = [
    `DATASET: ${state.gfb3Dsn}`,
    `COUNTRY: ${state.country}`,
    `SITE: ${state.siteName}`,
    `PI: ${state.piName}`,
    `CONTRIBUTOR: ${[state.contact.firstName, state.contact.middleName, state.contact.lastName].filter(Boolean).join(' ')}`,
    `CURATOR: Francisco Rivas`,
    `DATE RECEIVED: `,
    `DATE PROCESSED: ${new Date().toISOString().slice(0,10)}`,
    `--- SOURCE FORMAT ---`,
    `  ${state.filePath?(state.filePath.split(/[/\\]/).pop()):''}`,
    `--- PIVOT / RESTRUCTURING ---`, ``,
    `--- DUPLICATE RESOLUTION ---`, ``,
    `--- MISSING / INTERPOLATED DATA ---`, ``,
    `--- SPECIES ISSUES ---`, ``,
    `--- EXCLUSIONS ---`, ``,
    `--- NOTES ---`,
  ];
  if (state.validationReport) {
    state.validationReport.findings.filter(f=>f.severity==='Escalate').forEach(f=>{
      lines.push(`  [AUTO-FLAGGED] ${fmtRule(f.rule)} (${f.row_count} rows) — ${f.message}`);
    });
  }
  return lines.join('\n');
}

// ── Navigation ─────────────────────────────────────────────────────────────────
function renderNav() {
  const canBack = state.step > 0;
  let hint = '', nextLabel = t('nav.next'), nextDisabled = false;

  const lr = state.loadResult;
  if (state.mode === 'diagnose') {
    if (state.step === STEP.CONTACT) {
      const dsn = computeDsn();
      if (!state.contact.firstName.trim() || !state.contact.lastName.trim()) { hint = t('nav.hint.nameRequired'); nextDisabled = true; }
      else if (!state.countryName) { hint = t('nav.hint.selectCountry'); nextDisabled = true; }
      else if (!String(state.submitYear).match(/^\d{4}$/)) { hint = t('nav.hint.submitYear'); nextDisabled = true; }
      else if (!dsn) { hint = t('nav.hint.completeDsn'); nextDisabled = true; }
    }
    if (state.step === STEP.MODE && !state.mode) { hint = t('nav.hint.chooseMode'); nextDisabled = true; }
    if (state.step === STEP.LOAD) {
      if (!lr) { hint = t('nav.hint.loadFile'); nextDisabled = true; }
      else if (lr.gate_errors.length) { hint = t('nav.hint.fixStructure'); nextDisabled = true; }
    }
  } else {
    if (state.step === STEP.CONTACT) {
      const dsn = computeDsn();
      if (!state.contact.firstName.trim() || !state.contact.lastName.trim()) { hint = t('nav.hint.nameRequired'); nextDisabled = true; }
      else if (!state.countryName) { hint = t('nav.hint.selectCountry'); nextDisabled = true; }
      else if (!String(state.submitYear).match(/^\d{4}$/)) { hint = t('nav.hint.submitYear'); nextDisabled = true; }
      else if (!dsn) { hint = t('nav.hint.completeDsn'); nextDisabled = true; }
    }
    if (state.step === STEP.MODE && !state.mode) { hint = t('nav.hint.chooseMode'); nextDisabled = true; }
    if (state.step === STEP.LOAD) {
      if (!lr) { hint = t('nav.hint.loadFile'); nextDisabled = true; }
      else if (lr.gate_errors.length) { hint = t('nav.hint.fixStructure'); nextDisabled = true; }
    }
    if (state.step === STEP.FORMAT && !state.dataFormat) { hint = t('nav.hint.selectFormat'); nextDisabled = true; }

    if (state.step === STEP.INVENTORY) {
      const { ok, reason } = inventoryValid();
      if (!ok) { hint = reason; nextDisabled = true; }
      else if (state.dataFormat === 'wide' && state.wideStep === 0) { nextLabel = t('nav.nextYears'); }
      else if (state.dataFormat === 'wide') { nextLabel = t('nav.pivotContinue'); }
      else { nextLabel = t('nav.applyMapping'); }
    }

    if (state.step === STEP.STATUS) {
      if (state.statusMode === 'derive' && !state.deriveResult) { hint = t('nav.hint.runDerive'); nextDisabled = true; }
      else if (state.statusMode === 'column' && !statusColSelected()) { hint = t('nav.hint.selectStatusCol'); nextDisabled = true; }
      else { nextLabel = t('nav.validate'); }
    }

    if (state.step === STEP.VALIDATE) {
      nextLabel = t('nav.continueExport');
      const hasBlockers = state.validationReport?.findings?.some(f=>f.severity==='RequiresInput'||f.severity==='Escalate');
      if (hasBlockers) hint = t('nav.hint.resolveBeforeExport');
    }
  }

  const isLastStep = state.step === stepNames().length - 1;
  if (isLastStep) { nextLabel = t('nav.startOver'); nextDisabled = false; }

  return `
    <button class="btn btn-ghost" id="btn-prev" ${canBack?'':'disabled'}>${t('common.back')}</button>
    <span class="nav-hint">${esc(hint)}</span>
    <button class="btn btn-primary" id="btn-next" ${nextDisabled?'disabled':''}>${esc(nextLabel)}</button>`;
}

function inventoryValid() {
  if (state.dataFormat === 'wide') {
    if (state.wideStep === 0) {
      if (!state.fa.plotId.cols.length) return { ok: false, reason: t('nav.reason.plotIdWide') };
      if (!state.fa.treeId.cols.length) return { ok: false, reason: t('nav.reason.treeIdWide') };
      if (!state.wideDbhCols.length)    return { ok: false, reason: t('nav.reason.dbhColWide') };
      return { ok: true, reason: '' };
    }
    const minCensuses = state.censusType === 'single' ? 1 : 2;
    const valid = state.widePairs.filter(p => state.wideDbhCols.includes(p.source_column) && p.year > 0);
    if (valid.length < minCensuses) return { ok: false, reason: t('nav.reason.assignYears', { min: minCensuses }) };
    return { ok: true, reason: '' };
  }
  const fa = state.fa;
  if (!fa.plotId.cols.length) return { ok: false, reason: t('nav.reason.plotId') };
  if (!fa.treeId.cols.length) return { ok: false, reason: t('nav.reason.treeId') };
  if (!fa.dbh.col)            return { ok: false, reason: t('nav.reason.dbh') };
  if (!fa.yr.col)             return { ok: false, reason: t('nav.reason.yr') };
  return { ok: true, reason: '' };
}

function statusColSelected() {
  const fromInventory = cols().find(c => state.colRoles[c] === 'status');
  return !!(state.statusColOverride || fromInventory);
}

// ── Main render ────────────────────────────────────────────────────────────────
function render() {
  try { document.getElementById('step-indicator').innerHTML = renderStepIndicator(); } catch(e){}
  try {
    let html = '';
    if (state.mode === 'diagnose') {
      html = (diagnoseRender() || renderStep1)();
    } else {
      html = (harmonizeRender() || renderStep3)();
    }
    document.getElementById('main').innerHTML = html;
  } catch(e) {
    console.error('Render error:', e);
    document.getElementById('main').innerHTML = `<div style="color:red;padding:2rem;font-family:monospace;white-space:pre-wrap">${esc(t('error.render', { msg: e.message }))}\n${esc(e.stack)}</div>`;
  }
  try { document.getElementById('nav').innerHTML = renderNav(); } catch(e){}
  try { attachHandlers(); } catch(e) { console.error('Handler error:', e); }
  I18n.applyStaticLabels();
}

// ── Handlers ───────────────────────────────────────────────────────────────────
function attachHandlers() {
  // Step 0
  el('pick-file', e => e.addEventListener('click', async () => {
    clearError();
    const path = await openDialog({ multiple: false, filters: [{ name: t('common.dataFiles'), extensions: ['xlsx','xls','csv','tsv','parquet'] }] });
    if (!path) return;
    showLoading(t('loading.reading'));
    try {
      const result = await invoke('load_file', { path });
      state.filePath = path;
      state.loadResult = result;
      // Seed colRoles from suggestions
      state.colRoles = {};
      for (const s of result.suggested_mappings) {
        if (!s.suggested_gfb3_field) continue;
        const roleMap = { PlotId:'plot_id', TreeId:'tree_id', Yr:'yr', Dbh:'dbh', Species:'species', Status:'status' };
        const role = roleMap[s.suggested_gfb3_field];
        if (role) state.colRoles[s.source_column] = role;
      }
      state.plotIdOrder = []; state.treeIdOrder = [];
      state.widePairs = result.columns
        .map(c => { const m = c.match(/(\d{4})$/); return m ? { source_column: c, year: parseInt(m[1],10) } : null; })
        .filter(Boolean);
    } catch(e) { showError(String(e)); }
    finally { hideLoading(); render(); }
  }));

  // Step 1
  el('mode-harmonize', e => e.addEventListener('click', () => { state.mode = 'harmonize'; render(); }));
  el('mode-diagnose',  e => e.addEventListener('click', () => { state.mode = 'diagnose';  render(); }));

  // Step 2
  el('fmt-long', e => e.addEventListener('click', () => { state.dataFormat = 'long'; render(); }));
  el('fmt-wide', e => e.addEventListener('click', () => { state.dataFormat = 'wide'; render(); }));

  // Step 3: contact + DSN
  bind('f-firstname',  v => { state.contact.firstName = v; updateDsnPreview(); });
  bind('f-midname',    v => { state.contact.middleName = v; });
  bind('f-lastname',   v => { state.contact.lastName = v; updateDsnPreview(); });
  bindS('f-country',   v => { state.countryName = v; state.country = isoFromCountry(v).toUpperCase(); updateDsnPreview(); render(); });
  bind('f-submityear', v => { state.submitYear = v; updateDsnPreview(); });
  bind('f-site',       v => { state.siteName = v; });
  bind('f-pi',         v => { state.piName = v; });
  qsa('input[name="dbh-unit"]',   r => r.addEventListener('change', e => { state.dbhUnit = e.target.value; }));
  qsa('input[name="census-type"]',r => r.addEventListener('change', e => { state.censusType = e.target.value; updateDsnPreview(); }));

  // Step 4 long — single-column selectors
  qsa('.fa-sel', sel => sel.addEventListener('change', e => {
    const id = e.target.id, v = e.target.value;
    const map = { 'fa-dbh':'dbh', 'fa-yr':'yr', 'fa-species':'species',
                  'fa-lat':'lat', 'fa-lon':'lon', 'fa-pa':'pa', 'fa-status':'status' };
    if (map[id]) { state.fa[map[id]].col = v; render(); }
  }));
  // Step 4 long — prefix: update state + preview in-place (NO full render — avoids scroll jump)
  el('fa-plotid-prefix', inp => {
    inp.addEventListener('input', e => {
      state.fa.plotId.prefix = e.target.value;
      const prev = document.getElementById('plotid-preview');
      if (prev) {
        const p = concatPreview(state.fa.plotId.cols, state.fa.plotId.prefix);
        prev.style.display = '';
        prev.innerHTML = `→ <strong>${esc(p)}</strong>`;
      }
    });
  });
  // Step 4 — plot metadata lookup
  el('lookup-enabled', cb => cb.addEventListener('change', e => {
    state.plotLookup.enabled = e.target.checked;
    if (!e.target.checked) {
      // Clear lookup selections so they don't bleed into buildFieldExprs
      state.plotLookup.latCol = '';
      state.plotLookup.lonCol = '';
      state.plotLookup.paCol  = '';
    }
    render();
  }));
  el('pick-lookup-file', btn => btn.addEventListener('click', async () => {
    const path = await openDialog({ multiple: false, filters: [{ name: t('common.dataFiles'), extensions: ['xlsx','xls','csv','tsv','parquet'] }] });
    if (!path) return;
    showLoading(t('loading.lookup'));
    try {
      const columns = await invoke('preview_file', { path });
      state.plotLookup.filePath = path;
      state.plotLookup.columns  = columns;
      // Reset column selections when a new file is picked
      state.plotLookup.lookupKeyCol = '';
      state.plotLookup.latCol = '';
      state.plotLookup.lonCol = '';
      state.plotLookup.paCol  = '';
      render();
    } catch(e) { showError(String(e)); } finally { hideLoading(); }
  }));
  bindS('lookup-main-key', v => { state.plotLookup.mainKeyCol   = v; });
  bindS('lookup-lk-key',   v => { state.plotLookup.lookupKeyCol = v; });
  bindS('lookup-lat-col',  v => { state.plotLookup.latCol = v; render(); });
  bindS('lookup-lon-col',  v => { state.plotLookup.lonCol = v; render(); });
  bindS('lookup-pa-col',   v => { state.plotLookup.paCol  = v; render(); });

  bind('fa-lat-lit',  v => { state.fa.lat.literal = v; });
  bind('fa-lon-lit',  v => { state.fa.lon.literal = v; });
  bind('fa-pa-lit',   v => { state.fa.pa.literal  = v; });
  bindS('coord-format-sel', v => { state.coordFormat = v; render(); });
  el('open-map-btn', btn => btn.addEventListener('click', openMapModal));

  // Step 4 — multi-col add/remove (PlotID, TreeID)
  qsa('.fa-add-sel', sel => sel.addEventListener('change', e => {
    const key = e.target.dataset.fakey, val = e.target.value;
    if (val && !state.fa[key].cols.includes(val)) { state.fa[key].cols.push(val); }
    render();
  }));
  qsa('.fa-pill-remove', btn => btn.addEventListener('click', e => {
    const key = e.target.dataset.fakey, idx = parseInt(e.target.dataset.idx, 10);
    state.fa[key].cols.splice(idx, 1);
    render();
  }));

  // Step 4 wide sub-step 0 — DBH checkboxes
  qsa('.wide-dbh-chk', chk => chk.addEventListener('change', e => {
    const col = e.target.dataset.col;
    if (e.target.checked) { if (!state.wideDbhCols.includes(col)) state.wideDbhCols.push(col); }
    else { state.wideDbhCols = state.wideDbhCols.filter(c => c !== col); }
    render();
  }));
  // Step 4 wide — single field selectors (Species, Status, Lat, Lon, PA)
  qsa('.fa-wide-single', sel => sel.addEventListener('change', e => {
    const field = e.target.dataset.widefield, v = e.target.value;
    const map = { species:'species', status:'status', latitude:'lat', longitude:'lon', pa:'pa' };
    if (map[field]) { state.fa[map[field]].col = v; render(); }
  }));
  // Step 4 wide — year inputs
  qsa('.year-input', inp => inp.addEventListener('input', e => {
    const col = e.target.dataset.col, yr = parseInt(e.target.value, 10);
    state.widePairs = state.widePairs.filter(p => p.source_column !== col);
    if (!isNaN(yr) && yr > 0) state.widePairs.push({ source_column: col, year: yr });
    refreshNav();
  }));

  // Step 5: status mode tabs
  qsa('[data-field="status"][data-mode]', btn => btn.addEventListener('click', () => {
    state.statusMode = btn.dataset.mode;
    render();
  }));
  bindS('fm-status-col', v => { state.statusColOverride = v; refreshNav(); });
  qsa('.treatment-btn', btn => btn.addEventListener('click', async () => {
    state.disappearedTreatment = btn.dataset.treatment;
    render();
    // Auto re-derive so the treatment change takes effect immediately
    if (state.deriveResult) {
      clearError();
      showLoading(t('loading.reDerive'));
      try {
        state.deriveResult = await invoke('derive_status', { request: { disappeared_treatment: state.disappearedTreatment } });
        render();
      } catch(e) { showError(String(e)); } finally { hideLoading(); }
    }
  }));
  el('run-derive-btn', btn => btn.addEventListener('click', async () => {
    clearError();
    showLoading(t('loading.derive'));
    try {
      state.deriveResult = await invoke('derive_status', { request: { disappeared_treatment: state.disappearedTreatment } });
      render();
    } catch(e) { showError(String(e)); } finally { hideLoading(); }
  }));

  // Leaflet map
  el('map-modal-close',   btn => btn.addEventListener('click', closeMapModal));
  el('map-modal-confirm', btn => btn.addEventListener('click', confirmMapCoords));

  // Export
  el('pick-outdir', btn => btn.addEventListener('click', async () => {
    const dir = await openDialog({ directory: true });
    if (dir) { const e = document.getElementById('f-outdir'); if (e) e.value = dir; }
  }));
  el('do-export', btn => btn.addEventListener('click', doExport));

  attachNavHandlers();
}

function updateDsnPreview() {
  const dsn = computeDsn();
  const preview = document.getElementById('dsn-preview');
  if (preview) {
    preview.innerHTML = dsn
      ? esc(dsn)
      : `<span style="color:var(--text-muted);font-style:italic">${t('step3.dsnEmpty')}</span>`;
    state.gfb3Dsn = dsn;
  }
  refreshNav();
}

function attachDragReorder() {
  let dragSrc = null;
  qsa('[draggable="true"][data-orderkey]', el => {
    el.addEventListener('dragstart', e => { dragSrc = el; e.dataTransfer.effectAllowed = 'move'; });
    el.addEventListener('dragover',  e => { e.preventDefault(); e.dataTransfer.dropEffect = 'move'; });
    el.addEventListener('drop', e => {
      e.preventDefault();
      if (!dragSrc || dragSrc === el) return;
      const key  = el.dataset.orderkey;
      const from = parseInt(dragSrc.dataset.idx, 10);
      const to   = parseInt(el.dataset.idx, 10);
      const arr  = state[key];
      const [item] = arr.splice(from, 1);
      arr.splice(to, 0, item);
      render();
    });
  });
}

function refreshNav() {
  document.getElementById('nav').innerHTML = renderNav();
  attachNavHandlers();
}

function attachNavHandlers() {
  el('btn-prev', btn => btn.addEventListener('click', () => {
    if (state.step === STEP.INVENTORY && state.dataFormat === 'wide' && state.wideStep === 1) {
      state.wideStep = 0; clearError(); render(); return;
    }
    if (state.step > 0) { state.step--; clearError(); render(); }
  }));
  el('btn-next', btn => btn.addEventListener('click', async () => {
    // Last step: start over by reloading the page (clears all state cleanly)
    if (state.step === stepNames().length - 1) { window.location.reload(); return; }
    clearError();
    try { await advanceStep(); } catch(e) { showError(String(e)); }
  }));
}

// ── Step advance ───────────────────────────────────────────────────────────────
async function advanceStep() {
  if (state.step === STEP.CONTACT) collectStep3Dom();

  // Diagnose: after Load → validate → results
  if (state.mode === 'diagnose' && state.step === STEP.LOAD) {
    showLoading(t('loading.integrity'));
    try {
      await invoke('use_raw_as_gfb3');
      state.validationReport = await invoke('run_validation');
      state.step = DIAGNOSE_STEP_KEYS.length - 1; render();
    } finally { hideLoading(); }
    return;
  }

  // Harmonize only from here
  if (state.mode === 'diagnose') {
    if (state.step < stepNames().length - 1) { state.step++; render(); }
    return;
  }

  // Harmonize inventory wide sub-step 0 → 1
  if (state.mode === 'harmonize' && state.step === STEP.INVENTORY && state.dataFormat === 'wide' && state.wideStep === 0) {
    state.wideStep = 1;
    render();
    return;
  }

  // Harmonize inventory (long): apply field mapping
  if (state.mode === 'harmonize' && state.step === STEP.INVENTORY && state.dataFormat === 'long') {
    showLoading(t('loading.mapping'));
    try {
      const fields = buildFieldExprs();
      state.applyResult = await invoke('apply_fields_mapping', {
        request: {
          gfb3_dsn: state.gfb3Dsn,
          fields,
          dbh_unit: state.dbhUnit,
          status_remaps: [],
          metadata: {
            country:      state.country  || null,
            site:         state.siteName || null,
            pi:           state.piName   || null,
            dbh_unit:     state.dbhUnit,
            census_years: state.censusYears,
            census_type:  state.censusType,
          },
        },
      });
      state.deriveResult = null;
      // For single-census data, skip the Status step (no PrevYR/PrevDBH needed)
      state.step = state.censusType === 'single' ? STEP.VALIDATE : STEP.STATUS;
      if (state.censusType === 'single') {
        showLoading(t('loading.validate'));
        try { state.validationReport = await invoke('run_validation'); } finally { hideLoading(); }
      }
      render();
    } finally { hideLoading(); }
    return;
  }

  // Harmonize inventory (wide): pivot
  if (state.mode === 'harmonize' && state.step === STEP.INVENTORY && state.dataFormat === 'wide') {
    showLoading(t('loading.pivot'));
    try {
      const wideIdentityExprs = buildFieldExprs().filter(f =>
        !['DBH','YR'].includes(f.target_col)
      );
      state.applyResult = await invoke('apply_wide_mapping', {
        request: {
          gfb3_dsn:       state.gfb3Dsn,
          identity_exprs: wideIdentityExprs,
          dbh_pairs:      state.widePairs.filter(p=>state.wideDbhCols.includes(p.source_column)&&p.year>0),
          status_remaps:     [],
          metadata: {
            country:      state.country  || null,
            site:         state.siteName || null,
            pi:           state.piName   || null,
            dbh_unit:     state.dbhUnit,
            census_years: state.widePairs.filter(p=>p.year>0).map(p=>p.year),
            census_type:  state.censusType,
          },
        },
      });
      state.deriveResult = null;
      state.step = STEP.STATUS; render();
    } finally { hideLoading(); }
    return;
  }

  // Harmonize status → validate
  if (state.mode === 'harmonize' && state.step === STEP.STATUS) {
    if (state.statusMode === 'column') {
      // Re-apply with the status column included
      showLoading(t('loading.statusCol'));
      try {
        await invoke('apply_fields_mapping', {
          request: {
            gfb3_dsn: state.gfb3Dsn,
            fields:   buildFieldExprs(),
            dbh_unit: state.dbhUnit,
            status_remaps: [],
            metadata: { country: state.country||null, site: state.siteName||null, pi: state.piName||null, dbh_unit: state.dbhUnit, census_years: state.censusYears, census_type: state.censusType },
          },
        });
      } finally { hideLoading(); }
    }
    showLoading(t('loading.validate'));
    try {
      state.validationReport = await invoke('run_validation');
      state.step = STEP.VALIDATE; render();
    } finally { hideLoading(); }
    return;
  }

  if (state.step < stepNames().length - 1) { state.step++; render(); }
}

function collectStep3Dom() {
  const v = id => (document.getElementById(id)||{}).value || '';
  state.contact.firstName  = v('f-firstname') || state.contact.firstName;
  state.contact.middleName = v('f-midname')   || state.contact.middleName;
  state.contact.lastName   = v('f-lastname')  || state.contact.lastName;
  const ctryVal = v('f-country');
  if (ctryVal) { state.countryName = ctryVal; state.country = isoFromCountry(ctryVal).toUpperCase(); }
  state.submitYear = v('f-submityear') || state.submitYear;
  state.siteName   = v('f-site')  || state.siteName;
  state.piName     = v('f-pi')    || state.piName;
  const ru = document.querySelector('input[name="dbh-unit"]:checked');
  state.dbhUnit = ru ? ru.value : state.dbhUnit;
  const rc = document.querySelector('input[name="census-type"]:checked');
  state.censusType = rc ? rc.value : state.censusType;
  state.gfb3Dsn = computeDsn();
}

// ── Build field expressions from fa state ─────────────────────────────────────
function buildFieldExprs() {
  const fa = state.fa;
  const pl = state.plotLookup;
  const fields = [];

  // PlotID
  const plotCols = fa.plotId.cols;
  const prefix   = fa.plotId.prefix.trim();
  if (plotCols.length === 1 && !prefix) {
    fields.push({ kind:'column', target_col:'PlotID', source: plotCols[0] });
  } else if (plotCols.length >= 1) {
    fields.push({ kind:'concat', target_col:'PlotID', sources: plotCols, sep:'_', to_lower:true, prefix: prefix || null });
  }

  // TreeID — no auto-prepend; user selects exactly which columns to use
  const treeCols = fa.treeId.cols;
  if (treeCols.length === 1) {
    fields.push({ kind:'column', target_col:'TreeID', source: treeCols[0] });
  } else if (treeCols.length > 1) {
    fields.push({ kind:'concat', target_col:'TreeID', sources: treeCols, sep:'_', to_lower:true, prefix: null });
  }

  // Single-column measurement fields
  if (fa.dbh.col)     fields.push({ kind:'column', target_col:'DBH',     source: fa.dbh.col });
  if (fa.yr.col)      fields.push({ kind:'column', target_col:'YR',      source: fa.yr.col });
  if (fa.species.col) fields.push({ kind:'column', target_col:'Species', source: fa.species.col });

  // Lat/Lon/PA — lookup takes precedence over direct column/literal
  const lookupBase = pl.enabled && pl.filePath && pl.mainKeyCol && pl.lookupKeyCol
    ? { kind: 'lookup', lookup_path: pl.filePath, main_key: pl.mainKeyCol, lookup_key: pl.lookupKeyCol }
    : null;

  if (lookupBase && pl.latCol) {
    fields.push({ ...lookupBase, target_col: 'Latitude',  value_col: pl.latCol });
  } else if (fa.lat.col) {
    fields.push({ kind:'column',  target_col:'Latitude',  source: fa.lat.col });
  } else if (fa.lat.literal !== '') {
    fields.push({ kind:'literal', target_col:'Latitude',  value: fa.lat.literal });
  }

  if (lookupBase && pl.lonCol) {
    fields.push({ ...lookupBase, target_col: 'Longitude', value_col: pl.lonCol });
  } else if (fa.lon.col) {
    fields.push({ kind:'column',  target_col:'Longitude', source: fa.lon.col });
  } else if (fa.lon.literal !== '') {
    fields.push({ kind:'literal', target_col:'Longitude', value: fa.lon.literal });
  }

  if (lookupBase && pl.paCol) {
    fields.push({ ...lookupBase, target_col: 'PA', value_col: pl.paCol });
  } else if (fa.pa.col) {
    fields.push({ kind:'column',  target_col:'PA', source: fa.pa.col });
  } else if (fa.pa.literal !== '') {
    fields.push({ kind:'literal', target_col:'PA', value: fa.pa.literal });
  }

  // Status — only if column mode
  if (state.statusMode === 'column') {
    const stCol = state.statusColOverride || fa.status.col;
    if (stCol) fields.push({ kind:'column', target_col:'Status', source: stCol });
  }

  return fields;
}

// ── Export ─────────────────────────────────────────────────────────────────────
async function doExport() {
  clearError();
  const outDir  = (document.getElementById('f-outdir') ||{}).value||'';
  const base    = (document.getElementById('f-basename')||{}).value||state.gfb3Dsn||'dataset';
  const formats = [...document.querySelectorAll('input[name="fmt"]:checked')].map(e=>e.value);
  if (!outDir.trim()) { showError(t('error.chooseOutDir')); return; }
  if (!formats.length) { showError(t('error.chooseFormat')); return; }
  showLoading(t('loading.exporting'));
  try {
    const files = await invoke('export', { request: { output_dir: outDir, base_name: base, formats } });
    document.getElementById('export-result').innerHTML = `
      <div class="finding finding-clean" style="margin-top:1rem">
        <div class="finding-header"><span class="finding-rule">${t('step7.exportComplete')}</span><span class="badge badge-clean">${t('step7.files', { count: files.length })}</span></div>
        <ul style="margin:.4rem 0 0 1rem;font-size:.82rem">${files.map(f=>`<li style="font-family:monospace;margin-bottom:.2rem">${esc(f)}</li>`).join('')}</ul>
      </div>`;
  } catch(e) { showError(String(e)); } finally { hideLoading(); }
}

// ── Leaflet map ────────────────────────────────────────────────────────────────
let leafletMap = null, leafletMarker = null, pickedLatLon = null;

function openMapModal() {
  const modal = document.getElementById('map-modal');
  if (!modal) return;
  modal.classList.add('visible');
  if (!leafletMap && typeof L !== 'undefined') {
    leafletMap = L.map('map-container').setView([0, 0], 2);
    L.tileLayer('https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png', { attribution: '© OpenStreetMap contributors', maxZoom: 18 }).addTo(leafletMap);
    leafletMap.on('click', e => {
      const {lat, lng} = e.latlng;
      pickedLatLon = { lat: lat.toFixed(6), lon: lng.toFixed(6) };
      if (leafletMarker) leafletMarker.setLatLng(e.latlng);
      else leafletMarker = L.marker(e.latlng).addTo(leafletMap);
      document.getElementById('map-coords-display').textContent = t('map.coords', { lat: pickedLatLon.lat, lon: pickedLatLon.lon });
      document.getElementById('map-modal-confirm').disabled = false;
    });
  } else if (leafletMap) setTimeout(() => leafletMap.invalidateSize(), 50);
}
function closeMapModal() { document.getElementById('map-modal')?.classList.remove('visible'); }
function confirmMapCoords() {
  if (!pickedLatLon) return;
  state.fa.lat.literal = pickedLatLon.lat;
  state.fa.lon.literal = pickedLatLon.lon;
  closeMapModal(); render();
}

// ── Init ───────────────────────────────────────────────────────────────────────
console.log('=== Forest Data Harmonizer initializing ===');
I18n.init({ onChange: () => render() });
window.onerror = (msg, src, line) => { showError(t('error.js', { msg, src, line })); return false; };
window.addEventListener('unhandledrejection', ev => showError(t('error.generic', { reason: ev.reason })));

try {
  render();
  console.log('✓ Initial render complete');
} catch(e) {
  console.error('Initial render failed:', e);
  document.getElementById('main').innerHTML = `<div style="color:red;padding:2rem;font-family:monospace;white-space:pre-wrap">${esc(t('error.init', { msg: e.message, stack: e.stack }))}</div>`;
}
