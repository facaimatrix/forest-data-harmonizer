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
    throw new Error(t('error.tauriInvoke'));
  return invokeApi.invoke(cmd, args);
};
const openDialog = async (opts = {}) => {
  if (!dialogApi) await initTauri();
  if (!dialogApi || typeof dialogApi.open !== 'function') throw new Error(t('error.dialogOpen'));
  return dialogApi.open(opts);
};
const saveDialog = async (opts = {}) => {
  if (!dialogApi) await initTauri();
  if (!dialogApi || typeof dialogApi.save !== 'function') throw new Error(t('error.dialogSave'));
  return dialogApi.save(opts);
};

// ── i18n ──────────────────────────────────────────────────────────────────────
const t = (key, params) => I18n.t(key, params);
const appLocale = () => I18n.getLocale();

// ── Constants ─────────────────────────────────────────────────────────────────
const HARMONIZE_STEP_KEYS = ['steps.setup', 'steps.mode', 'steps.format', 'steps.inventory', 'steps.status', 'steps.species', 'steps.validate', 'steps.export'];
const DIAGNOSE_STEP_KEYS  = ['steps.setup', 'steps.mode', 'steps.diagnose'];

// Harmonize: Load+Dataset → Mode → Format → Inventory → Status → Species → Validate → Export
const STEP = {
  SETUP: 0,   // Load file + contact / dataset metadata (one pane)
  MODE: 1,
  FORMAT: 2,
  INVENTORY: 3,
  STATUS: 4,
  SPECIES: 5,
  VALIDATE: 6,
  EXPORT: 7,
};

function harmonizeRender() {
  return [renderSetup, renderStep1, renderStep2, renderStep4, renderStep5, renderSpeciesStep, renderStep6, renderStep7][state.step] || renderSetup;
}

function diagnoseRender() {
  return [renderSetup, renderStep1, renderDiagnoseStep2][state.step] || renderSetup;
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
  {n:'Canada',iso:'CAN'},{n:'Central African Republic',iso:'CAF'},{n:'Chad',iso:'TCD'},{n:'Chile',iso:'CHL'},
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
  { role: 'pa',        label: 'PA',        type: 'number',  multi: false, required: false, desc: 'Plot area in hectares (e.g. 1.0). Used for per-hectare density and, for fixed-area plots, to compute EXPAN = 1/PA. Can be a constant if all plots are the same size.' },
  { role: 'expan',     label: 'EXPAN',     type: 'number',  multi: false, required: false, desc: 'Expansion factor: trees per hectare represented by one measured tree. For fixed-area plots EXPAN = 1/PA. For other designs, enter a constant or leave blank to fill later. Not the same as plot-level TPH (which is n/PA or the sum of EXPAN).' },
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
  expan:     'EXPAN (expansion factor)',
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
  contact: { firstName: '', middleName: '', lastName: '', email: '' },
  countryName: '',    // full name, ISO3 derived from COUNTRIES lookup
  country:     '',    // ISO3 (derived)
  submitYear:  new Date().getFullYear(),
  censusType:  'multi', // 'single' | 'multi'
  gfb3Dsn:   '',
  siteName:  '',
  piName:    '',
  piEmail:   '',
  piSameAsContact: false,
  curatorName: '',
  dbhUnit:   'cm',
  censusYears: [],
  // Expansion factor (EXPAN) — tree-level weight
  fixedArea: true,          // default Yes
  expanMode: 'later',       // when !fixedArea: 'constant' | 'later'
  constantExpan: '',

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
  statusRemaps: {},      // source label → GFB3 code
  statusVocab: null,     // rows from get_status_vocab
  statusVocabCol: null,  // column vocab was loaded for
  statusVocabLoading: false,
  statusColOverride: '',

  // Species / TNRS
  speciesResults: null,
  speciesResolutions: {}, // original → resolved
  speciesMessage: null,
  speciesSkipped: false,

  // Export options
  keepAliveOnly: true,
  saveInSourceFolder: false,
  exportOutDir: '',

  // Wide format (kept from original)
  columnMappings: [],
  widePairs: [],
  applyResult: null,
  validationReport: null,
  diagnosticReport: null,

  // Free-access workspaces (not tied to wizard nav)
  workspaceView: 'workflow', // 'workflow' | 'map' | 'convert'
  mapView: {
    latCol: '',
    lonCol: '',
    labelCol: '',
    symbolCol: '',
    crs: 'EPSG:4326',
    utmZone: '18',
    points: [],
    truncated: false,
    status: '',
    autoTried: false,
    autoPlotted: false,
  },
  convertView: {
    inputPath: '',
    outDir: '',
    baseName: '',
    formats: { csv: true, tsv: false, parquet: true, xlsx: false },
    result: null,
  },
};

// ── Utilities ─────────────────────────────────────────────────────────────────
function esc(s) {
  if (s == null) return '';
  const d = document.createElement('div');
  d.textContent = String(s);
  return d.innerHTML;
}
function dirnameOf(path) {
  if (!path) return '';
  const trimmed = String(path).replace(/[/\\]+$/, '');
  const i = Math.max(trimmed.lastIndexOf('/'), trimmed.lastIndexOf('\\'));
  return i < 0 ? '' : trimmed.slice(0, i);
}
function sourceFileDir() {
  return dirnameOf(state.filePath);
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
function usableCols() {
  return cols().filter(c => c && String(c).trim() && !String(c).startsWith('_unnamed_'));
}

function freshFa() {
  return {
    plotId:  { cols: [], prefix: '' },
    treeId:  { cols: [] },
    pa:      { col: '', literal: '' },
    lat:     { col: '', literal: '' },
    lon:     { col: '', literal: '' },
    species: { col: '' },
    dbh:     { col: '' },
    yr:      { col: '' },
    status:  { col: '' },
  };
}

/** Keep only source columns that still exist in the loaded file. */
function pruneFaToColumns(columns) {
  const set = new Set(columns || []);
  state.fa.plotId.cols = state.fa.plotId.cols.filter(c => set.has(c));
  state.fa.treeId.cols = state.fa.treeId.cols.filter(c => set.has(c));
  for (const key of ['pa', 'lat', 'lon', 'species', 'dbh', 'yr', 'status']) {
    if (state.fa[key].col && !set.has(state.fa[key].col)) state.fa[key].col = '';
  }
}

function contactDisplayName() {
  return [state.contact.firstName, state.contact.middleName, state.contact.lastName]
    .filter(Boolean)
    .join(' ')
    .trim();
}

function syncPiFromContact() {
  if (!state.piSameAsContact) return;
  state.piName = contactDisplayName();
  state.piEmail = state.contact.email || '';
  const piEl = document.getElementById('f-pi');
  const emEl = document.getElementById('f-pi-email');
  if (piEl) piEl.value = state.piName;
  if (emEl) emEl.value = state.piEmail;
}

function buildMetadataPayload() {
  return {
    country:        state.country  || null,
    site:           state.siteName || null,
    pi:             state.piName   || null,
    pi_email:       state.piEmail  || null,
    contact:        contactDisplayName() || null,
    contact_email:  state.contact.email || null,
    dbh_unit:       state.dbhUnit,
    census_years:   state.censusYears,
    census_type:    state.censusType,
  };
}

/** Prefer exact GFB3-like header names when fuzzy suggestion missed them. */
function seedExactCanonicalCols(columns) {
  const list = columns || [];
  const norm = c => String(c).toLowerCase().replace(/[\s-]+/g, '_');
  const findExact = (aliases) => list.find(c => aliases.includes(norm(c)));
  if (!state.fa.treeId.cols.length) {
    const hit = findExact(['treeid', 'tree_id']);
    if (hit) state.fa.treeId.cols = [hit];
  }
  if (!state.fa.plotId.cols.length) {
    const hit = findExact(['plotid', 'plot_id']);
    if (hit) state.fa.plotId.cols = [hit];
  }
  if (!state.fa.lat.col) {
    const hit = findExact(['lat', 'latitude', 'latitud', 'y_lat', 'coord_y', 'coords_y']);
    if (hit) state.fa.lat.col = hit;
  }
  if (!state.fa.lon.col) {
    const hit = findExact(['lon', 'long', 'longitude', 'longitud', 'lng', 'x_lon', 'x_long', 'coord_x', 'coords_x']);
    if (hit) state.fa.lon.col = hit;
  }
  if (!state.fa.pa.col) {
    const hit = findExact(['pa', 'plot_area', 'plotarea', 'area_ha', 'plot_ha']);
    if (hit) state.fa.pa.col = hit;
  }
}

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

// ── Step 0: Load file + Contact / Dataset metadata ─────────────────────────────
function renderSetup() {
  const lr = state.loadResult;
  const gate = lr && lr.gate_errors.length
    ? `<div class="gate-errors"><h3>${t('step0.gateTitle')}</h3><ul>${lr.gate_errors.map(e=>`<li>${esc(fmtGateError(e))}</li>`).join('')}</ul></div>`
    : (lr ? `<p class="file-chosen" style="margin-top:.5rem">${t('step0.structureOk')}</p>` : '');

  const preview = lr ? `
    <div class="preview-wrap">
      <div class="preview-meta">${t('step0.previewMeta', { rows: lr.row_count.toLocaleString(), cols: lr.columns.length, shown: lr.preview_rows.length })}</div>
      <div style="overflow-x:auto">
        <table><thead><tr>${lr.columns.map(c=>`<th>${esc(c)}</th>`).join('')}</tr></thead>
        <tbody>${lr.preview_rows.map(row=>`<tr>${row.map(v=>v==null?`<td class="null-cell">${t('common.null')}</td>`:`<td>${esc(v)}</td>`).join('')}</tr>`).join('')}</tbody></table>
      </div>
    </div>` : '';

  const c = state.contact;
  const dsn = computeDsn();
  const iso = isoFromCountry(state.countryName).toUpperCase();
  const countryOpts = COUNTRIES.map(ct =>
    `<option value="${esc(ct.n)}" ${state.countryName===ct.n?'selected':''}>${esc(I18n.countryName(ct.iso, ct.n))}</option>`
  ).join('');

  return `<div class="step-content">
    <h2>${t('setup.title')}</h2>
    <p class="step-desc">${t('setup.desc')}</p>

    <p class="section-heading" style="margin-top:0">${t('step0.title')}</p>
    <p class="step-desc" style="margin-top:0">${t('step0.desc')}</p>
    <button class="btn btn-primary btn-lg" id="pick-file">${t('step0.pick')}</button>
    ${state.filePath ? `<p class="file-chosen" style="margin-top:.75rem;max-width:600px">${esc(state.filePath)}</p>` : ''}
    ${gate}${preview}

    <p class="section-heading">${t('step3.title')}</p>
    <p class="step-desc" style="margin-top:0">${t('step3.desc')}</p>

    <p class="section-heading" style="margin-top:1rem;font-size:.85rem">${t('step3.personHeading')}</p>
    <div class="form-grid" style="max-width:560px">
      <label>${t('step3.firstName')} <span class="required-mark">*</span></label>
      <input type="text" id="f-firstname" value="${esc(c.firstName)}" placeholder="${t('step3.phFirst')}" />

      <label>${t('step3.middleName')}</label>
      <input type="text" id="f-midname" value="${esc(c.middleName)}" placeholder="${t('common.optional')}" />

      <label>${t('step3.lastName')} <span class="required-mark">*</span></label>
      <input type="text" id="f-lastname" value="${esc(c.lastName)}" placeholder="${t('step3.phLast')}" />

      <label>${t('step3.contactEmail')}</label>
      <input type="email" id="f-contact-email" value="${esc(c.email)}" placeholder="${t('step3.phEmail')}" />

      <label>${t('step3.curator')} <span class="required-mark">*</span></label>
      <input type="text" id="f-curator" value="${esc(state.curatorName)}" placeholder="${t('step3.phCurator')}" />
    </div>

    <p class="section-heading">${t('step3.provenanceHeading')}</p>
    <div class="form-grid" style="max-width:560px">
      <label>${t('step3.country')} <span class="required-mark">*</span></label>
      <div>
        <select id="f-country" style="min-width:260px;padding:.4rem .6rem;border:1px solid var(--border);border-radius:var(--radius);font-size:.875rem;font-family:inherit;background:var(--surface)">
          <option value="">${t('common.selectCountry')}</option>
          ${countryOpts}
        </select>
        <span id="country-iso" style="font-family:monospace;font-size:.82rem;color:var(--green-dark);margin-left:.6rem">${iso ? esc(iso) : ''}</span>
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
      <div style="display:flex;align-items:center;gap:.75rem;flex-wrap:wrap">
        <input type="text" id="f-pi" value="${esc(state.piName)}" placeholder="${t('step3.phPi')}"
          style="flex:1;min-width:10rem" ${state.piSameAsContact ? 'disabled' : ''} />
        <label style="display:inline-flex;align-items:center;gap:.35rem;font-size:.82rem;color:var(--text-muted);white-space:nowrap;font-weight:400;cursor:pointer">
          <input type="checkbox" id="f-pi-same" ${state.piSameAsContact ? 'checked' : ''} />
          ${t('step3.piSameAsContact')}
        </label>
      </div>

      <label>${t('step3.piEmail')}</label>
      <input type="email" id="f-pi-email" value="${esc(state.piEmail)}" placeholder="${t('step3.phEmail')}"
        ${state.piSameAsContact ? 'disabled' : ''} />

      <label>${t('step3.dbhUnit')}</label>
      <div class="radio-group">
        <label><input type="radio" name="dbh-unit" value="cm" ${state.dbhUnit==='cm'?'checked':''} /> ${t('step3.unitCm')}</label>
        <label><input type="radio" name="dbh-unit" value="mm" ${state.dbhUnit==='mm'?'checked':''} /> ${t('step3.unitMm')} <span style="color:var(--text-muted)">${t('step3.mmHint')}</span></label>
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

// ── Step 4 helpers ────────────────────────────────────────────────────────────
function colSelect(id, value, extra='') {
  const opts = `<option value="">${t('common.none')}</option>` +
    usableCols().map(c => `<option value="${esc(c)}" ${value===c?'selected':''}>${esc(c)}</option>`).join('');
  return `<select class="fa-sel" id="${id}" style="min-width:200px;padding:.35rem .5rem;border:1px solid var(--border);border-radius:var(--radius);font-size:.83rem;font-family:inherit;background:var(--surface)" ${extra}>${opts}</select>`;
}

function multiColPills(faKey) {
  const arr = Array.isArray(state.fa[faKey].cols) ? state.fa[faKey].cols : [];
  const available = usableCols().filter(c => !arr.includes(c));
  const pills = arr.map((c,i) =>
    `<span class="fa-pill" data-fa-key="${faKey}" data-idx="${i}">${esc(c)}<button type="button" class="fa-pill-remove" data-fa-key="${faKey}" data-idx="${i}" title="${t('common.remove')}">×</button></span>`
  ).join('');
  const addSel = available.length
    ? `<select class="fa-add-sel" data-fa-key="${faKey}" style="padding:.3rem .5rem;border:1px solid var(--border);border-radius:var(--radius);font-size:.8rem;font-family:inherit;background:var(--surface);color:var(--text-muted)">
        <option value="">${t('common.addColumn')}</option>
        ${available.map(c=>`<option value="${esc(c)}">${esc(c)}</option>`).join('')}
       </select>`
    : '';
  return `<div class="fa-pills" style="display:flex;align-items:center;gap:.35rem;flex-wrap:wrap">${pills}${addSel}</div>`;
}

function concatFormula(cols_, prefix = '') {
  const parts = [...(prefix ? [String(prefix).trim().toLowerCase()] : []), ...cols_.map(c => String(c).toLowerCase())];
  return parts.filter(Boolean).join('_') || '…';
}

/**
 * Build PlotID/TreeID samples the same way as Rust concat (lowercase, "_" join).
 * Uses the first preview rows in order (not unique-only) so adding a column
 * visibly changes the samples even when values repeat.
 */
function concatSampleValues(cols_, prefix = '', max = 5) {
  const lr = state.loadResult;
  if (!lr || (!cols_.length && !(prefix && String(prefix).trim()))) return [];
  const idxs = cols_.map(c => lr.columns.indexOf(c));
  // If a selected column is missing from the frame, still show what we can
  const pfx = prefix && String(prefix).trim() ? String(prefix).trim() : '';
  const out = [];
  for (const row of lr.preview_rows) {
    const parts = [];
    if (pfx) parts.push(pfx);
    let anyPresent = false;
    for (const i of idxs) {
      if (i < 0) {
        parts.push('?');
        continue;
      }
      anyPresent = true;
      const v = row[i];
      if (v == null || String(v).trim() === '') {
        // Keep a placeholder so multi-column joins stay visible (a_ / a_b)
        parts.push('');
      } else {
        parts.push(String(v).trim());
      }
    }
    if (!pfx && !anyPresent) continue;
    // Trim trailing empties from a single trailing null, but keep internal gaps
    while (parts.length > 1 && parts[parts.length - 1] === '') parts.pop();
    const built = parts.join('_').toLowerCase();
    if (!built) continue;
    out.push(built);
    if (out.length >= max) break;
  }
  return out;
}

function concatPreviewHtml(cols_, prefix = '') {
  const samples = concatSampleValues(cols_, prefix, 5);
  const formula = concatFormula(cols_, prefix);
  if (!cols_.length && !(prefix && String(prefix).trim())) return '';
  const sampleLine = samples.length
    ? `<div><strong>${samples.map(esc).join('</strong>, <strong>')}</strong></div>`
    : '';
  return `→ ${sampleLine}<div class="fa-preview-meta">${esc(formula)}</div>`;
}

function sampleForCol(c) {
  const vals = sampleVals(c);
  return vals.length ? `<span class="fa-sample">${vals.slice(0,3).map(esc).join(', ')}</span>` : '';
}

// ── Step 4: Field Assignment ───────────────────────────────────────────────────
function renderPlotMetaSection() {
  const fa = state.fa;
  const pl = state.plotLookup;
  const allCols = usableCols();
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
    <div class="fa-grid" style="margin-top:.6rem">
      ${renderFixedAreaSection()}
    </div>
  </div>`;
}

function renderFixedAreaSection() {
  const yes = state.fixedArea;
  const mode = state.expanMode || 'later';
  return `
    <div class="fa-row" style="margin-top:1rem;border-top:1px solid var(--border);padding-top:.85rem">
      <div class="fa-left">
        <div class="fa-label" style="display:flex;align-items:center;gap:.35rem">
          ${t('fields.fixedArea')}
          <button type="button" class="field-help-btn" id="fixed-area-help-btn" aria-label="${esc(t('fields.fixedAreaHelpAria'))}" title="${esc(t('fields.fixedAreaHelpAria'))}">ⓘ</button>
        </div>
        <div class="help-tooltip" id="fixed-area-help">${t('fields.fixedAreaHelp')}</div>
        <div class="fa-desc">${t('fields.fixedAreaDesc')}</div>
      </div>
      <div class="fa-right">
        <div class="radio-group" style="display:flex;gap:1rem;flex-wrap:wrap;margin-bottom:.55rem">
          <label style="display:flex;align-items:center;gap:.35rem;cursor:pointer;font-size:.88rem">
            <input type="radio" name="fixed-area" value="yes" ${yes?'checked':''} /> ${t('common.yes')}
          </label>
          <label style="display:flex;align-items:center;gap:.35rem;cursor:pointer;font-size:.88rem">
            <input type="radio" name="fixed-area" value="no" ${!yes?'checked':''} /> ${t('common.no')}
          </label>
        </div>
        ${yes ? `<p style="font-size:.82rem;color:var(--text-muted);margin:0">${t('fields.expanFixedNote')}</p>` : `
        <div style="display:flex;flex-direction:column;gap:.55rem">
          <label style="display:flex;align-items:flex-start;gap:.4rem;cursor:pointer;font-size:.85rem">
            <input type="radio" name="expan-mode" value="constant" ${mode==='constant'?'checked':''} style="margin-top:.2rem" />
            <span>
              ${t('fields.expanEnter')}
              <input type="number" step="any" min="0" id="fa-expan-lit" class="fa-text-input"
                value="${esc(state.constantExpan)}" placeholder="${t('fields.phExpan')}"
                style="width:110px;margin-left:.45rem" ${mode!=='constant'?'disabled':''} />
            </span>
          </label>
          <label style="display:flex;align-items:center;gap:.4rem;cursor:pointer;font-size:.85rem">
            <input type="radio" name="expan-mode" value="later" ${mode==='later'?'checked':''} />
            ${t('fields.expanLater')}
          </label>
          <p style="font-size:.78rem;color:var(--text-muted);margin:0">${t('fields.expanVarNote')}</p>
        </div>`}
      </div>
    </div>`;
}

function renderStep4() {
  if (state.dataFormat === 'wide') return renderStep4Wide();

  const allCols = usableCols();
  if (!allCols.length) return `<div class="step-content"><h2>${t('step4.title')}</h2><p>${t('step4.noColumns')}</p></div>`;

  const fa = state.fa;
  const plotPreviewHtml = concatPreviewHtml(fa.plotId.cols, fa.plotId.prefix);
  const treePreviewHtml = concatPreviewHtml(fa.treeId.cols);

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
        ${(fa.plotId.cols.length || fa.plotId.prefix) ? `<div class="fa-preview" id="plotid-preview">${plotPreviewHtml}</div>` : `<div class="fa-preview" id="plotid-preview" style="display:none"></div>`}
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
        ${fa.treeId.cols.length ? `<div class="fa-preview" id="treeid-preview">${treePreviewHtml}</div>` : ''}
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
  const allCols = usableCols();
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
  const previewHtml = concatPreviewHtml(fa[faKey].cols, faKey === 'plotId' ? fa[faKey].prefix : '');
  const previewId = faKey === 'plotId' ? 'plotid-preview' : (faKey === 'treeId' ? 'treeid-preview' : '');
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
      ${(fa[faKey].cols.length || (faKey === 'plotId' && fa.plotId.prefix))
        ? `<div class="fa-preview"${previewId ? ` id="${previewId}"` : ''}>${previewHtml}</div>`
        : ''}
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
  const mappedCol = state.fa.status.col;
  // Inventory already chose a Status column → remap only (no re-pick).
  // Otherwise derive status from census structure.
  if (mappedCol) {
    state.statusMode = 'column';
    if (!state.statusColOverride) state.statusColOverride = mappedCol;
    return `<div class="step-content">
      <h2>${t('step5.title')}</h2>
      <p class="step-desc">${t('step5.remapOnlyDesc', { col: mappedCol })}</p>
      <div class="field-card" style="margin-top:.75rem">
        <p style="font-size:.83rem;margin-bottom:.75rem">${t('step5.columnPre')} <strong><code>${esc(mappedCol)}</code></strong></p>
        ${renderStatusRemapTable()}
      </div>
    </div>`;
  }

  state.statusMode = 'derive';
  const dr = state.deriveResult;
  return `<div class="step-content">
    <h2>${t('step5.title')}</h2>
    <p class="step-desc">${t('step5.desc')}</p>
    <div class="field-card" style="margin-top:.75rem">
      <div class="info-box" style="margin-bottom:1rem">
        <strong>${t('step5.rulesTitle')}</strong><br>
        • ${t('step5.rule1')}<br>
        • ${t('step5.rule2')}<br>
        • ${t('step5.rule3')}<br>
        • ${t('step5.rule4')}
      </div>
      ${dr && dr.summary.disappeared_tree_count > 0 ? renderDisappearedBox() : ''}
      ${dr ? renderDeriveStats(dr) : `<p style="font-size:.83rem;color:var(--text-muted)">${t('step5.clickDerive')}</p>`}
      <button class="btn btn-primary" id="run-derive-btn" style="margin-top:1rem">${t('step5.deriveBtn')}</button>
    </div>
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

function activeStatusCol() {
  return state.statusColOverride || state.fa.status.col || cols().find(c => state.colRoles[c] === 'status') || '';
}

function guessStatusCode(label) {
  const s = String(label ?? '').trim().toLowerCase();
  if (/^(0|alive|live|a|v|vivo|viv)$/.test(s)) return '0';
  if (/^(1|dead|d|died|mort|muerto|m)$/.test(s)) return '1';
  if (/^(2|recruit|new|ingrowth|recruta|nuevo)$/.test(s)) return '2';
  if (/^(9|missing|miss|na|n\/a|null|nd|\.)$/.test(s)) return '9';
  if (/^[0129]$/.test(s)) return s;
  return '9';
}

function statusRemapOptions(selected) {
  return [
    ['0', t('step5.code0')],
    ['1', t('step5.code1')],
    ['2', t('step5.code2')],
    ['9', t('step5.code9')],
  ].map(([v, lbl]) => `<option value="${v}" ${selected===v?'selected':''}>${esc(lbl)}</option>`).join('');
}

function renderStatusRemapTable() {
  const col = activeStatusCol();
  if (!col) return '';
  if (state.statusVocabLoading || state.statusVocabCol !== col) {
    return `<p style="font-size:.83rem;color:var(--text-muted);margin-top:1rem">${t('step5.remapLoading')}</p>`;
  }
  if (!state.statusVocab || !state.statusVocab.length) {
    return `<p style="font-size:.83rem;color:var(--text-muted);margin-top:1rem">${t('step5.remapEmpty')}</p>`;
  }
  const rows = state.statusVocab.map(row => {
    const cur = state.statusRemaps[row.source_value] ?? row.current_target ?? guessStatusCode(row.source_value);
    return `<tr>
      <td><code>${esc(row.source_value)}</code></td>
      <td style="text-align:right;color:var(--text-muted)">${row.row_count.toLocaleString()}</td>
      <td><select class="status-remap-sel" data-source="${esc(row.source_value)}">${statusRemapOptions(cur)}</select></td>
    </tr>`;
  }).join('');
  return `
    <div style="margin-top:1.25rem">
      <h4 style="font-size:.9rem;margin-bottom:.35rem">${t('step5.remapTitle')}</h4>
      <p style="font-size:.78rem;color:var(--text-muted);margin-bottom:.6rem">${t('step5.remapDesc')}</p>
      <table class="preview-table" style="width:100%;font-size:.83rem">
        <thead><tr><th>${t('step5.remapSource')}</th><th>${t('step5.remapCount')}</th><th>${t('step5.remapTarget')}</th></tr></thead>
        <tbody>${rows}</tbody>
      </table>
    </div>`;
}

async function loadStatusVocab(col) {
  if (!col) {
    state.statusVocab = null;
    state.statusVocabCol = null;
    state.statusVocabLoading = false;
    return;
  }
  if (state.statusVocabLoading && state.statusVocabCol === col) return;
  state.statusVocabLoading = true;
  try {
    const rows = await invoke('get_status_vocab', { column: col });
    state.statusVocab = Array.isArray(rows) ? rows : [];
    state.statusVocabCol = col;
    for (const row of state.statusVocab) {
      if (state.statusRemaps[row.source_value] == null) {
        state.statusRemaps[row.source_value] = guessStatusCode(row.source_value);
      }
    }
  } catch (e) {
    state.statusVocab = [];
    state.statusVocabCol = col;
    throw e;
  } finally {
    state.statusVocabLoading = false;
  }
}

function buildStatusRemaps() {
  return Object.entries(state.statusRemaps)
    .filter(([, tgt]) => tgt != null && tgt !== '')
    .map(([source_value, target_code]) => ({ source_value, target_code, note: null }));
}

function statusRemapsComplete() {
  // Already GFB3-coded labels still count; require vocab loaded when using a column
  if (!activeStatusCol()) return false;
  if (state.statusVocabLoading || state.statusVocabCol !== activeStatusCol()) return false;
  if (!state.statusVocab || !state.statusVocab.length) return true; // empty column — allow continue
  return state.statusVocab.every(row => {
    const tgt = state.statusRemaps[row.source_value];
    return tgt != null && tgt !== '';
  });
}

// ── Step: Species / TNRS ───────────────────────────────────────────────────────
function renderSpeciesStep() {
  const rows = state.speciesResults || [];
  if (!rows.length && !state.speciesMessage && !state.speciesSkipped) {
    return `<div class="step-content">
      <h2>${t('species.title')}</h2>
      <p class="step-desc">${t('species.desc')}</p>
      <p style="font-size:.83rem;color:var(--text-muted)">${t('species.clickResolve')}</p>
      <button class="btn btn-primary" id="run-tnrs-btn" style="margin-top:1rem">${t('species.resolveBtn')}</button>
    </div>`;
  }
  if (state.speciesSkipped) {
    return `<div class="step-content">
      <h2>${t('species.title')}</h2>
      <p class="step-desc">${esc(state.speciesMessage || t('species.skipped'))}</p>
      <p style="font-size:.83rem;color:var(--text-muted)">${t('species.skipHint')}</p>
    </div>`;
  }
  const needsReview = r => !!(r.ambiguous || (r.near_duplicates && r.near_duplicates.length));
  const sorted = [...rows].sort((a, b) => Number(needsReview(b)) - Number(needsReview(a)));
  const reviewCount = sorted.filter(needsReview).length;

  const rowHtml = r => {
    const amb = needsReview(r);
    const cur = state.speciesResolutions[r.original] ?? r.best_accepted ?? r.original;
    const opts = [];
    opts.push(`<option value="${esc(r.original)}" ${cur===r.original?'selected':''}>${esc(r.original)} ${t('species.keepOriginal')}</option>`);
    if (r.best_accepted && r.best_accepted !== r.original) {
      opts.push(`<option value="${esc(r.best_accepted)}" ${cur===r.best_accepted?'selected':''}>${esc(r.best_accepted)} ★</option>`);
    }
    (r.matches || []).forEach(m => {
      const name = m.accepted_name || m.name_matched;
      if (!name || name === r.original || name === r.best_accepted) return;
      if (opts.some(o => o.includes(`value="${esc(name)}"`))) return;
      const score = m.overall_score != null ? ` (${Number(m.overall_score).toFixed(2)})` : '';
      opts.push(`<option value="${esc(name)}" ${cur===name?'selected':''}>${esc(name)}${score}</option>`);
    });
    const near = (r.near_duplicates || []).length
      ? `<div class="species-near">${t('species.nearDup')}: ${(r.near_duplicates||[]).map(esc).join(', ')}</div>`
      : '';
    return `<tr class="${amb?'species-amb':''}">
      <td><code>${esc(r.original)}</code>${near}</td>
      <td><select class="species-res-sel" data-original="${esc(r.original)}">${opts.join('')}</select></td>
      <td>${amb ? `<span class="badge badge-input">${t('species.ambiguous')}</span>` : `<span class="badge badge-clean">${t('species.ok')}</span>`}</td>
    </tr>`;
  };

  const reviewRows = sorted.filter(needsReview).map(rowHtml).join('');
  const okRows = sorted.filter(r => !needsReview(r)).map(rowHtml).join('');

  return `<div class="step-content">
    <h2>${t('species.title')}</h2>
    <p class="step-desc">${t('species.desc')}</p>
    ${state.speciesMessage ? `<div class="info-box" style="margin-bottom:1rem">${esc(state.speciesMessage)}</div>` : ''}
    <div style="display:flex;gap:.5rem;margin-bottom:1rem;flex-wrap:wrap">
      <button class="btn btn-ghost" id="run-tnrs-btn">${t('species.resolveBtn')}</button>
      <button class="btn btn-ghost" id="tnrs-accept-best">${t('species.acceptBest')}</button>
    </div>
    ${reviewCount ? `
      <p class="section-heading" style="margin-top:0">${t('species.reviewHeading', { count: reviewCount })}</p>
      <table class="preview-table" style="width:100%;font-size:.83rem;margin-bottom:1.5rem">
        <thead><tr><th>${t('species.colOriginal')}</th><th>${t('species.colResolved')}</th><th>${t('species.colFlag')}</th></tr></thead>
        <tbody>${reviewRows}</tbody>
      </table>` : ''}
    ${okRows ? `
      <p class="section-heading" style="margin-top:0">${t('species.okHeading')}</p>
      <table class="preview-table" style="width:100%;font-size:.83rem">
        <thead><tr><th>${t('species.colOriginal')}</th><th>${t('species.colResolved')}</th><th>${t('species.colFlag')}</th></tr></thead>
        <tbody>${okRows}</tbody>
      </table>` : ''}
  </div>`;
}

// ── Step 6: Validation + diagnostic report ─────────────────────────────────────
function renderStep6() {
  const report = state.validationReport;
  const diag = state.diagnosticReport;
  if (!report && !diag) {
    return `<div class="step-content"><h2>${t('step6.title')}</h2><p class="step-desc">${t('step6.loading')}</p></div>`;
  }

  const findings = report?.findings || [];
  const bySev = sev => findings.filter(f => f.severity === sev).length;
  const badges = [
    { sev: 'AutoDrop', label: t('sev.autoDrop'), cls: 'auto' },
    { sev: 'AutoRecode', label: t('sev.autoRecode'), cls: 'recode' },
    { sev: 'RequiresInput', label: t('sev.needsReview'), cls: 'input' },
    { sev: 'Escalate', label: t('sev.escalate'), cls: 'escalate' },
  ].filter(b => bySev(b.sev) > 0).map(b =>
    `<span class="badge badge-${b.cls}">${bySev(b.sev)} ${b.label}</span>`
  ).join('');

  const findHtml = findings.length === 0
    ? `<div class="finding finding-clean"><div class="finding-header"><span class="finding-rule">${t('step6.allPassed')}</span><span class="badge badge-clean">${t('step6.clean')}</span></div><p class="finding-message">${t('step6.noIssues')}</p></div>`
    : findings.map(f => {
        const cls = { AutoDrop: 'auto', AutoRecode: 'recode', RequiresInput: 'input', Escalate: 'escalate' }[f.severity] || 'recode';
        return `<div class="finding finding-${cls}">
          <div class="finding-header">
            <span class="finding-rule">${esc(fmtRule(f.rule))}</span>
            <span class="badge badge-${cls}">${esc(fmtSev(f.severity))}</span>
            <span class="finding-count">${t('step6.rowCount', { count: f.row_count.toLocaleString() })}</span>
          </div>
          <p class="finding-message">${esc(fmtValidationMessage(f))}</p>
          <div class="finding-action">${t('common.action')}: <strong>${esc(fmtAction(f.action))}</strong></div>
        </div>`;
      }).join('');

  const blockers = findings.some(f => f.severity === 'RequiresInput' || f.severity === 'Escalate');
  const diagHtml = diag?.html
    ? `<div class="diag-panel">${diag.html}</div>`
    : '';
  const exportBtns = diag
    ? `<div class="diag-export-bar">
        <button class="btn btn-primary" id="export-diag-pdf">${t('step6.exportPdf')}</button>
        <button class="btn btn-ghost" id="export-diag-html">${t('step6.exportHtml')}</button>
      </div>`
    : '';

  return `<div class="step-content">
    <h2>${t('step6.title')}</h2>
    <p class="step-desc">${t('step6.desc')}</p>
    ${exportBtns}
    ${diagHtml}
    <h3 class="section-heading" style="margin-top:1.5rem">${t('step6.integrityHeading')}</h3>
    <p class="step-desc">${t('step6.integrityDesc')}</p>
    ${badges ? `<div class="findings-summary">${badges}</div>` : ''}
    ${findHtml}
    ${blockers ? `<div class="gate-errors" style="margin-top:1rem"><h3>${t('step6.reviewRequired')}</h3><p style="font-size:.85rem;color:var(--error)">${t('step6.reviewHint')}</p></div>` : ''}
  </div>`;
}

// ── Step 7: Export ─────────────────────────────────────────────────────────────
function renderStep7() {
  const isSingle = state.censusType === 'single';
  return `<div class="step-content">
    <h2>${t('step7.title')}</h2>
    <p class="step-desc">${isSingle ? t('step7.descSingle') : t('step7.descMulti')}</p>
    <div class="info-box" style="margin-bottom:1rem">
      ${isSingle ? t('step7.gfb2SingleNote') : t('step7.gfb2MultiNote')}
      <div style="margin-top:.5rem">${t('step7.formatNote')}</div>
    </div>
    <label style="display:flex;align-items:flex-start;gap:.5rem;margin-bottom:1rem;font-size:.88rem;cursor:pointer">
      <input type="checkbox" id="keep-alive-only" ${state.keepAliveOnly?'checked':''} style="margin-top:.2rem" />
      <span>${t('step7.keepAliveOnly')}</span>
    </label>
    <div class="form-grid" style="max-width:640px;margin-bottom:1rem">
      <label>${t('step7.outDir')}</label>
      <div>
        <input type="text" id="f-outdir" value="${esc(state.exportOutDir)}" placeholder="${t('step7.phOutDir')}" style="width:100%;margin-bottom:.55rem" readonly />
        <div style="display:flex;gap:.65rem;align-items:center;flex-wrap:wrap">
          <label style="display:flex;align-items:center;gap:.4rem;font-size:.875rem;cursor:pointer;white-space:nowrap">
            <input type="checkbox" id="save-in-source-folder" ${state.saveInSourceFolder?'checked':''} ${sourceFileDir()?'':'disabled'} />
            <span>${t('step7.saveInSourceFolder')}</span>
          </label>
          <span style="color:var(--text-muted);font-size:.85rem">${t('common.or')}</span>
          <button class="btn btn-ghost" id="pick-outdir">${t('common.browse')}</button>
        </div>
      </div>
      <label>${t('step7.baseName')}</label>
      <input type="text" id="f-basename" value="${esc(state.gfb3Dsn || 'dataset')}" />
    </div>
    <p class="section-heading" style="margin-top:0">${t('step7.formats')}</p>
    <div class="export-formats">
      <label class="format-card"><input type="checkbox" name="fmt" value="csv"     checked /> ${t('step7.fmtCsv')}</label>
      <label class="format-card"><input type="checkbox" name="fmt" value="parquet" checked /> ${t('step7.fmtParquet')}</label>
      <label class="format-card"><input type="checkbox" name="fmt" value="xlsx"    checked /> ${t('step7.fmtXlsx')}</label>
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
  const diag = state.diagnosticReport;
  if (!report && !diag) return `<div class="step-content"><h2>${t('diagnose.running')}</h2></div>`;
  const findings = report?.findings || [];
  const bySev = sev => findings.filter(f => f.severity === sev).length;
  const badges = ['AutoDrop', 'AutoRecode', 'RequiresInput', 'Escalate']
    .filter(s => bySev(s) > 0)
    .map(s => {
      const cls = { AutoDrop: 'auto', AutoRecode: 'recode', RequiresInput: 'input', Escalate: 'escalate' }[s];
      return `<span class="badge badge-${cls}">${bySev(s)} ${fmtSev(s)}</span>`;
    }).join('');

  const findHtml = findings.length === 0
    ? `<div class="finding finding-clean"><div class="finding-header"><span class="finding-rule">${t('step6.allPassed')}</span><span class="badge badge-clean">${t('step6.clean')}</span></div><p class="finding-message">${t('diagnose.allPassedMsg')}</p></div>`
    : findings.map(f => {
        const cls = { AutoDrop: 'auto', AutoRecode: 'recode', RequiresInput: 'input', Escalate: 'escalate' }[f.severity] || 'recode';
        return `<div class="finding finding-${cls}"><div class="finding-header"><span class="finding-rule">${esc(fmtRule(f.rule))}</span><span class="badge badge-${cls}">${esc(fmtSev(f.severity))}</span><span class="finding-count">${t('step6.rowCount', { count: f.row_count.toLocaleString() })}</span></div><p class="finding-message">${esc(fmtValidationMessage(f))}</p><div class="finding-action">${t('common.action')}: <strong>${esc(fmtAction(f.action))}</strong></div></div>`;
      }).join('');

  const diagHtml = diag?.html ? `<div class="diag-panel">${diag.html}</div>` : '';
  const exportBtns = diag
    ? `<div class="diag-export-bar">
        <button class="btn btn-primary" id="export-diag-pdf">${t('step6.exportPdf')}</button>
        <button class="btn btn-ghost" id="export-diag-html">${t('step6.exportHtml')}</button>
      </div>`
    : '';

  return `<div class="step-content">
    <h2>${t('diagnose.title')}</h2>
    <p style="font-size:.83rem;color:var(--text-muted);margin-bottom:1rem">
      ${t('diagnose.fileMeta', {
        name: esc(state.filePath ? (state.filePath.split(/[/\\]/).pop()) : ''),
        rows: (state.loadResult?.row_count ?? 0).toLocaleString(),
        cols: (state.loadResult?.columns ?? []).length,
      })}
    </p>
    ${exportBtns}
    ${diagHtml}
    <h3 class="section-heading" style="margin-top:1.5rem">${t('step6.integrityHeading')}</h3>
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
function fmtGateError(e) {
  if (typeof e === 'string') return e;
  const code = e.code || '';
  const key = `gate.${code}`;
  return t(key, { name: e.name || '', count: e.count ?? 0 });
}

function fmtValidationMessage(f) {
  const count = f.row_count ?? 0;
  const valuesMatch = (f.message || '').match(/found: ([^)]+)\)/);
  const values = valuesMatch ? valuesMatch[1] : '';
  const keyMap = {
    DuplicateTreeWithinPlotYear: 'validation.duplicate',
    UnknownStatus: 'validation.unknownStatus',
    DeadTreeHasDbh: 'validation.deadHasDbh',
    RecruitMissingDbh: 'validation.recruitMissingDbh',
    OrphanDeadFirstCensus: 'validation.orphanDead',
    RecruitAtMinYear: 'validation.recruitMinYear',
    NoPersistentTreeId: 'validation.noPersistentTreeId',
  };
  const key = keyMap[f.rule];
  if (key) return t(key, { count, values });
  return f.message || '';
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
    `${t('curation.dataset')} ${state.gfb3Dsn}`,
    `${t('curation.country')} ${state.country}`,
    `${t('curation.site')} ${state.siteName}`,
    `${t('curation.pi')} ${state.piName}`,
    `${t('curation.contributor')} ${[state.contact.firstName, state.contact.middleName, state.contact.lastName].filter(Boolean).join(' ')}`,
    `${t('curation.curator')} ${state.curatorName.trim()}`,
    `${t('curation.dateReceived')} `,
    `${t('curation.dateProcessed')} ${new Date().toISOString().slice(0,10)}`,
    t('curation.sectionSource'),
    `  ${state.filePath?(state.filePath.split(/[/\\]/).pop()):''}`,
    t('curation.sectionPivot'), ``,
    t('curation.sectionDuplicate'), ``,
    t('curation.sectionMissing'), ``,
    t('curation.sectionSpecies'), ``,
    t('curation.sectionExclusions'), ``,
    t('curation.sectionNotes'),
  ];
  if (state.validationReport) {
    state.validationReport.findings.filter(f=>f.severity==='Escalate').forEach(f=>{
      lines.push(`  ${t('curation.autoFlagged')} ${fmtRule(f.rule)} (${f.row_count} ${t('common.rows')}) — ${fmtValidationMessage(f)}`);
    });
  }
  return lines.join('\n');
}

// ── Navigation ─────────────────────────────────────────────────────────────────
function renderNav() {
  const canBack = state.step > 0;
  let hint = '', nextLabel = t('nav.next'), nextDisabled = false;

  const lr = state.loadResult;
  if (state.step === STEP.SETUP) {
    if (!lr) { hint = t('nav.hint.loadFile'); nextDisabled = true; }
    else if (lr.gate_errors.length) { hint = t('nav.hint.fixStructure'); nextDisabled = true; }
    else {
      const dsn = computeDsn();
      if (!state.contact.firstName.trim() || !state.contact.lastName.trim()) { hint = t('nav.hint.nameRequired'); nextDisabled = true; }
      else if (!state.curatorName.trim()) { hint = t('nav.hint.curatorRequired'); nextDisabled = true; }
      else if (!state.countryName) { hint = t('nav.hint.selectCountry'); nextDisabled = true; }
      else if (!String(state.submitYear).match(/^\d{4}$/)) { hint = t('nav.hint.submitYear'); nextDisabled = true; }
      else if (!dsn) { hint = t('nav.hint.completeDsn'); nextDisabled = true; }
    }
  }
  if (state.mode === 'diagnose') {
    if (state.step === STEP.MODE && !state.mode) { hint = t('nav.hint.chooseMode'); nextDisabled = true; }
  } else {
    if (state.step === STEP.MODE && !state.mode) { hint = t('nav.hint.chooseMode'); nextDisabled = true; }
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
      else if (state.statusMode === 'column' && !statusRemapsComplete()) { hint = t('nav.hint.mapStatus'); nextDisabled = true; }
      else { nextLabel = t('nav.next'); }
    }

    if (state.step === STEP.SPECIES) {
      nextLabel = t('nav.validate');
    }

    if (state.step === STEP.VALIDATE) {
      nextLabel = t('nav.continueExport');
      const hasBlockers = state.validationReport?.findings?.some(f=>f.severity==='RequiresInput'||f.severity==='Escalate');
      if (hasBlockers) hint = t('nav.hint.resolveBeforeExport');
    }
  }

  const isLastStep = state.step === stepNames().length - 1;
  if (isLastStep) { nextLabel = t('nav.done'); nextDisabled = true; }

  return `
    <button class="btn btn-ghost" id="btn-prev" ${canBack?'':'disabled'}>${t('common.back')}</button>
    <span class="nav-hint">${esc(hint)}</span>
    ${isLastStep ? '' : `<button class="btn btn-primary" id="btn-next" ${nextDisabled?'disabled':''}>${esc(nextLabel)}</button>`}`;
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
  return !!activeStatusCol();
}

// ── Main render ────────────────────────────────────────────────────────────────
function render() {
  const utilView = state.workspaceView === 'map' || state.workspaceView === 'convert';
  try {
    document.body.classList.toggle('map-workspace', state.workspaceView === 'map');
    document.body.classList.toggle('convert-workspace', state.workspaceView === 'convert');
    document.querySelectorAll('.workspace-tab').forEach(btn => {
      btn.classList.toggle('active', btn.dataset.view === state.workspaceView);
      if (btn.dataset.view === 'workflow') btn.textContent = t('workspace.workflow');
      if (btn.dataset.view === 'map') btn.textContent = t('workspace.map');
      if (btn.dataset.view === 'convert') btn.textContent = t('workspace.convert');
    });
  } catch (e) {}
  try {
    document.getElementById('step-indicator').innerHTML =
      utilView ? '' : renderStepIndicator();
  } catch (e) {}
  try {
    let html = '';
    if (state.workspaceView === 'map') {
      maybeAutoSeedMapCols();
      html = renderMapView();
    } else if (state.workspaceView === 'convert') {
      html = renderConvertView();
    } else if (state.mode === 'diagnose') {
      html = (diagnoseRender() || renderStep1)();
    } else {
      html = (harmonizeRender() || renderSetup)();
    }
    document.getElementById('main').innerHTML = html;
  } catch(e) {
    console.error('Render error:', e);
    document.getElementById('main').innerHTML = `<div style="color:red;padding:2rem;font-family:monospace;white-space:pre-wrap">${esc(t('error.render', { msg: e.message }))}\n${esc(e.stack)}</div>`;
  }
  try {
    document.getElementById('nav').innerHTML =
      utilView ? '' : renderNav();
  } catch(e){}
  try { attachHandlers(); } catch(e) { console.error('Handler error:', e); }
  I18n.applyStaticLabels();
  if (state.workspaceView === 'map') {
    initPlotMapLeaflet();
    const mv = state.mapView;
    if (mv.latCol && mv.lonCol && !mv.points.length && !mv.autoPlotted) {
      mv.autoPlotted = true;
      loadAndPlotMapPoints();
    }
  }
  if (state.mode === 'harmonize' && state.step === STEP.STATUS && state.fa.status.col) {
    const col = activeStatusCol();
    if (col && state.statusVocabCol !== col && !state.statusVocabLoading) {
      loadStatusVocab(col)
        .then(() => render())
        .catch(e => { showError(String(e)); render(); });
    }
  }
}

// ── Handlers ───────────────────────────────────────────────────────────────────
function attachHandlers() {
  // Step 0
  el('pick-file', e => e.addEventListener('click', async () => {
    clearError();
    const path = await openDialog({ multiple: false, filters: [{ name: t('common.dataFiles'), extensions: ['xlsx','xls','csv','tsv','txt','parquet'] }] });
    if (!path) return;
    showLoading(t('loading.reading'));
    try {
      const result = await invoke('load_file', { path });
      state.filePath = path;
      state.loadResult = result;
      // New file → clear prior field picks (stale "PlotID" etc. breaks concat mapping)
      state.fa = freshFa();
      state.applyResult = null;
      state.validationReport = null;
      state.diagnosticReport = null;
      state.deriveResult = null;
      state.statusVocab = null;
      state.statusVocabCol = null;
      state.speciesResults = null;
      state.speciesResolutions = {};
      // Seed colRoles from suggestions
      state.colRoles = {};
      for (const s of result.suggested_mappings) {
        if (!s.suggested_gfb3_field) continue;
        const roleMap = {
          PlotId: 'plot_id', TreeId: 'tree_id', Yr: 'yr', Dbh: 'dbh',
          Species: 'species', Status: 'status',
          Latitude: 'lat', Longitude: 'lon', PA: 'pa',
        };
        const role = roleMap[s.suggested_gfb3_field];
        if (role) state.colRoles[s.source_column] = role;
        if (role === 'yr' && !state.fa.yr.col) state.fa.yr.col = s.source_column;
        if (role === 'status' && !state.fa.status.col) state.fa.status.col = s.source_column;
        if (role === 'dbh' && !state.fa.dbh.col) state.fa.dbh.col = s.source_column;
        if (role === 'species' && !state.fa.species.col) state.fa.species.col = s.source_column;
        if (role === 'plot_id' && !state.fa.plotId.cols.length) state.fa.plotId.cols = [s.source_column];
        if (role === 'tree_id' && !state.fa.treeId.cols.length) state.fa.treeId.cols = [s.source_column];
        if (role === 'lat' && !state.fa.lat.col) state.fa.lat.col = s.source_column;
        if (role === 'lon' && !state.fa.lon.col) state.fa.lon.col = s.source_column;
        if (role === 'pa' && !state.fa.pa.col) state.fa.pa.col = s.source_column;
      }
      pruneFaToColumns(result.columns);
      seedExactCanonicalCols(result.columns);
      state.plotIdOrder = []; state.treeIdOrder = [];
      state.mapView.autoTried = false;
      state.mapView.points = [];
      state.mapView.status = '';
      state.mapView.autoPlotted = false;
      // Seed map cols from field assignment / suggestions
      if (!state.mapView.latCol && state.fa.lat.col) state.mapView.latCol = state.fa.lat.col;
      if (!state.mapView.lonCol && state.fa.lon.col) state.mapView.lonCol = state.fa.lon.col;
      state.widePairs = result.columns
        .map(c => { const m = c.match(/(\d{4})$/); return m ? { source_column: c, year: parseInt(m[1],10) } : null; })
        .filter(Boolean);
    } catch(e) { showError(String(e)); }
    finally { hideLoading(); collectStep3Dom(); render(); }
  }));

  // Step 1
  el('mode-harmonize', e => e.addEventListener('click', () => { state.mode = 'harmonize'; render(); }));
  el('mode-diagnose',  e => e.addEventListener('click', () => { state.mode = 'diagnose';  render(); }));

  // Step 2
  el('fmt-long', e => e.addEventListener('click', () => { state.dataFormat = 'long'; render(); }));
  el('fmt-wide', e => e.addEventListener('click', () => { state.dataFormat = 'wide'; render(); }));

  // Step 3: contact + DSN
  bind('f-firstname',  v => { state.contact.firstName = v; updateDsnPreview(); syncPiFromContact(); });
  bind('f-midname',    v => { state.contact.middleName = v; syncPiFromContact(); });
  bind('f-lastname',   v => { state.contact.lastName = v; updateDsnPreview(); syncPiFromContact(); });
  bind('f-contact-email', v => { state.contact.email = v; syncPiFromContact(); });
  bind('f-curator',    v => { state.curatorName = v; refreshNav(); });
  // Country: update in place — never full-render on change. Native <select>
  // typeahead fires change on each letter match; re-rendering steals focus
  // (often onto Next) while the user is still searching.
  el('f-country', sel => {
    sel.addEventListener('keydown', e => {
      if (e.key === 'Enter') e.preventDefault();
    });
    sel.addEventListener('change', e => {
      const v = e.target.value;
      state.countryName = v;
      state.country = isoFromCountry(v).toUpperCase();
      const isoEl = document.getElementById('country-iso');
      if (isoEl) isoEl.textContent = state.country || '';
      updateDsnPreview();
    });
  });
  bind('f-submityear', v => { state.submitYear = v; updateDsnPreview(); });
  bind('f-site',       v => { state.siteName = v; });
  bind('f-pi',         v => { if (!state.piSameAsContact) state.piName = v; });
  bind('f-pi-email',   v => { if (!state.piSameAsContact) state.piEmail = v; });
  el('f-pi-same', chk => chk.addEventListener('change', e => {
    state.piSameAsContact = !!e.target.checked;
    if (state.piSameAsContact) syncPiFromContact();
    render();
  }));
  qsa('input[name="dbh-unit"]',   r => r.addEventListener('change', e => { state.dbhUnit = e.target.value; }));
  qsa('input[name="census-type"]',r => r.addEventListener('change', e => {
    state.censusType = e.target.value;
    if (state.censusType === 'single') state.dataFormat = 'long';
    updateDsnPreview();
  }));

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
        const html = concatPreviewHtml(state.fa.plotId.cols, state.fa.plotId.prefix);
        if (html) {
          prev.style.display = '';
          prev.innerHTML = html;
        } else {
          prev.style.display = 'none';
          prev.innerHTML = '';
        }
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
    const path = await openDialog({ multiple: false, filters: [{ name: t('common.dataFiles'), extensions: ['xlsx','xls','csv','tsv','txt','parquet'] }] });
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

  // Fixed-area / EXPAN
  qsa('input[name="fixed-area"]', r => r.addEventListener('change', e => {
    state.fixedArea = e.target.value === 'yes';
    render();
  }));
  qsa('input[name="expan-mode"]', r => r.addEventListener('change', e => {
    state.expanMode = e.target.value;
    render();
  }));
  bind('fa-expan-lit', v => { state.constantExpan = v; });
  el('fixed-area-help-btn', btn => btn.addEventListener('click', e => {
    e.preventDefault();
    e.stopPropagation();
    const tip = document.getElementById('fixed-area-help');
    if (tip) tip.classList.toggle('visible');
  }));

  // Map workspace toolbar
  bindS('map-lat-col', v => { state.mapView.latCol = v; });
  bindS('map-lon-col', v => { state.mapView.lonCol = v; });
  bindS('map-label-col', v => { state.mapView.labelCol = v; });
  bindS('map-symbol-col', v => {
    state.mapView.symbolCol = v;
    if (state.mapView.points.length && state.mapView.latCol && state.mapView.lonCol) {
      loadAndPlotMapPoints();
    } else if (state.mapView.points.length) {
      drawPlotMapPoints(state.mapView.points);
    }
  });
  bindS('map-crs', v => {
    state.mapView.crs = v;
    state.mapView.points = [];
    state.mapView.autoPlotted = false;
    state.mapView.status = '';
    render();
  });
  bind('map-utm-zone', v => { state.mapView.utmZone = v; });
  el('map-plot-btn', btn => btn.addEventListener('click', () => loadAndPlotMapPoints()));
  el('map-save-html', btn => btn.addEventListener('click', () => savePlotMapHtml()));

  // Format conversion workspace
  el('convert-pick-input', btn => btn.addEventListener('click', async () => {
    clearError();
    const path = await openDialog({
      multiple: false,
      filters: [{ name: t('common.dataFiles'), extensions: ['xlsx', 'xls', 'csv', 'tsv', 'txt', 'parquet'] }],
    });
    if (!path) return;
    const cv = state.convertView;
    cv.inputPath = path;
    cv.baseName = stemOf(path);
    if (!cv.outDir) cv.outDir = dirnameOf(path);
    cv.result = null;
    render();
  }));
  el('convert-pick-outdir', btn => btn.addEventListener('click', async () => {
    clearError();
    const dir = await openDialog({ directory: true });
    if (!dir) return;
    state.convertView.outDir = dir;
    render();
  }));
  el('convert-basename', inp => inp.addEventListener('input', e => {
    state.convertView.baseName = e.target.value;
  }));
  ['csv', 'tsv', 'parquet', 'xlsx'].forEach(fmt => {
    el(`convert-fmt-${fmt}`, chk => chk.addEventListener('change', e => {
      state.convertView.formats[fmt] = e.target.checked;
    }));
  });
  el('convert-run', btn => btn.addEventListener('click', async () => {
    clearError();
    const cv = state.convertView;
    if (!cv.inputPath) { showError(t('convert.needInput')); return; }
    if (!cv.outDir) { showError(t('convert.needOutDir')); return; }
    const formats = Object.entries(cv.formats).filter(([, on]) => on).map(([k]) => k);
    if (!formats.length) { showError(t('convert.needFormat')); return; }
    showLoading(t('convert.loading'));
    try {
      const result = await invoke('convert_file_formats', {
        request: {
          input_path: cv.inputPath,
          output_dir: cv.outDir,
          base_name: (cv.baseName || '').trim() || null,
          formats,
        },
      });
      cv.result = result;
      render();
    } catch (e) {
      showError(String(e));
    } finally {
      hideLoading();
    }
  }));

  // Step 4 — multi-col add/remove (PlotID, TreeID)
  qsa('.fa-add-sel', sel => sel.addEventListener('change', e => {
    const elSel = e.currentTarget;
    const key = elSel.dataset.faKey;
    const val = elSel.value;
    if (!key || !val || !state.fa[key]) return;
    // Only block the synthetic target name when it is not a real source column
    const isRealCol = usableCols().includes(val);
    if (!isRealCol && key === 'plotId' && val === 'PlotID') return;
    if (!isRealCol && key === 'treeId' && val === 'TreeID') return;
    if (!Array.isArray(state.fa[key].cols)) state.fa[key].cols = [];
    if (!state.fa[key].cols.includes(val)) {
      state.fa[key].cols = state.fa[key].cols.concat([val]);
    }
    render();
  }));
  qsa('.fa-pill-remove', btn => btn.addEventListener('click', e => {
    e.preventDefault();
    e.stopPropagation();
    const elBtn = e.currentTarget;
    const key = elBtn.dataset.faKey;
    const idx = parseInt(elBtn.dataset.idx, 10);
    if (!key || !state.fa[key] || !Array.isArray(state.fa[key].cols)) return;
    if (Number.isNaN(idx) || idx < 0 || idx >= state.fa[key].cols.length) return;
    state.fa[key].cols = state.fa[key].cols.filter((_, i) => i !== idx);
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

  // Step 5: status (derive when no inventory Status; remap when Status was mapped)
  qsa('.status-remap-sel', sel => sel.addEventListener('change', e => {
    state.statusRemaps[e.target.dataset.source] = e.target.value;
    refreshNav();
  }));
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
  el('save-in-source-folder', cb => cb.addEventListener('change', e => {
    state.saveInSourceFolder = e.target.checked;
    if (state.saveInSourceFolder) {
      const dir = sourceFileDir();
      if (!dir) {
        state.saveInSourceFolder = false;
        e.target.checked = false;
        showError(t('error.noSourceFolder'));
        return;
      }
      state.exportOutDir = dir;
    }
    const out = document.getElementById('f-outdir');
    if (out) out.value = state.exportOutDir;
  }));
  el('pick-outdir', btn => btn.addEventListener('click', async () => {
    const dir = await openDialog({ directory: true });
    if (!dir) return;
    state.exportOutDir = dir;
    state.saveInSourceFolder = false;
    const out = document.getElementById('f-outdir');
    if (out) out.value = dir;
    const cb = document.getElementById('save-in-source-folder');
    if (cb) cb.checked = false;
  }));
  el('keep-alive-only', cb => cb.addEventListener('change', e => { state.keepAliveOnly = e.target.checked; }));
  el('do-export', btn => btn.addEventListener('click', doExport));

  el('export-diag-pdf', btn => btn.addEventListener('click', () => exportDiagnostic('pdf')));
  el('export-diag-html', btn => btn.addEventListener('click', () => exportDiagnostic('html')));

  // Species / TNRS
  el('run-tnrs-btn', btn => btn.addEventListener('click', async () => {
    clearError();
    showLoading(t('loading.tnrs'));
    try {
      const res = await invoke('resolve_species_tnrs');
      state.speciesResults = res.results || [];
      state.speciesSkipped = !!res.skipped;
      state.speciesMessage = res.message || null;
      state.speciesResolutions = {};
      for (const r of state.speciesResults) {
        state.speciesResolutions[r.original] = r.best_accepted || r.original;
      }
      render();
    } catch (e) { showError(String(e)); }
    finally { hideLoading(); }
  }));
  el('tnrs-accept-best', btn => btn.addEventListener('click', () => {
    for (const r of (state.speciesResults || [])) {
      if (r.best_accepted) state.speciesResolutions[r.original] = r.best_accepted;
    }
    render();
  }));
  qsa('.species-res-sel', sel => sel.addEventListener('change', e => {
    state.speciesResolutions[e.target.dataset.original] = e.target.value;
  }));

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
    // Single-census skips Format — go Mode ← Inventory
    if (state.step === STEP.INVENTORY && state.censusType === 'single') {
      state.step = STEP.MODE; clearError(); render(); return;
    }
    // Species ← Status; Validate ← Species
    if (state.step > 0) { state.step--; clearError(); render(); }
  }));
  el('btn-next', btn => btn.addEventListener('click', async () => {
    clearError();
    try { await advanceStep(); } catch(e) { showError(String(e)); }
  }));
}

// ── Step advance ───────────────────────────────────────────────────────────────
async function advanceStep() {
  if (state.step === STEP.SETUP) collectStep3Dom();

  // Diagnose: after Mode → validate → results
  if (state.mode === 'diagnose' && state.step === STEP.MODE) {
    showLoading(t('loading.integrity'));
    try {
      await invoke('use_raw_as_gfb3');
      const result = await invoke('run_validation', { locale: appLocale() });
      state.validationReport = result.validation ?? result;
      state.diagnosticReport = result.diagnostic ?? null;
      state.step = DIAGNOSE_STEP_KEYS.length - 1; render();
    } finally { hideLoading(); }
    return;
  }

  if (state.mode === 'diagnose') {
    if (state.step < stepNames().length - 1) { state.step++; render(); }
    return;
  }

  // Single-census: skip Format (always long)
  if (state.step === STEP.MODE) {
    if (state.censusType === 'single') {
      state.dataFormat = 'long';
      state.step = STEP.INVENTORY;
      render();
      return;
    }
  }

  // After Format for multi, or when leaving Format
  // (handled by default increment)

  // Harmonize inventory wide sub-step 0 → 1
  if (state.step === STEP.INVENTORY && state.dataFormat === 'wide' && state.wideStep === 0) {
    state.wideStep = 1;
    render();
    return;
  }

  // Harmonize inventory (long): apply field mapping
  if (state.mode === 'harmonize' && state.step === STEP.INVENTORY && state.dataFormat === 'long') {
    // Drop stale picks that are not in the loaded file
    pruneFaToColumns(cols());
    const fields = buildFieldExprs();
    if (!fields.some(f => f.target_col === 'PlotID')) {
      showError(t('nav.reason.plotId'));
      return;
    }
    if (!fields.some(f => f.target_col === 'TreeID')) {
      showError(t('nav.reason.treeId'));
      return;
    }
    if (!fields.some(f => f.target_col === 'YR')) {
      showError(t('nav.reason.yr'));
      return;
    }
    showLoading(t('loading.mapping'));
    try {
      state.applyResult = await invoke('apply_fields_mapping', {
        request: {
          gfb3_dsn: state.gfb3Dsn,
          fields,
          dbh_unit: state.dbhUnit,
          status_remaps: [],
          metadata: buildMetadataPayload(),
        },
      });
      state.deriveResult = null;
      const hasStatusCol = !!state.fa.status.col;
      if (hasStatusCol) {
        state.statusMode = 'column';
        state.statusColOverride = state.fa.status.col;
        showLoading(t('loading.statusVocab'));
        try {
          await loadStatusVocab(state.fa.status.col);
        } catch (e) {
          showError(String(e));
        } finally {
          hideLoading();
        }
        state.step = STEP.STATUS;
      } else if (state.censusType === 'single') {
        state.step = STEP.SPECIES;
      } else {
        state.statusMode = 'derive';
        state.step = STEP.STATUS;
      }
      render();
    } finally { hideLoading(); }
    return;
  }

  // Harmonize inventory (wide): pivot
  if (state.step === STEP.INVENTORY && state.dataFormat === 'wide') {
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
            ...buildMetadataPayload(),
            census_years: state.widePairs.filter(p=>p.year>0).map(p=>p.year),
          },
        },
      });
      state.deriveResult = null;
      if (state.fa.status.col) {
        state.statusMode = 'column';
        state.statusColOverride = state.fa.status.col;
        showLoading(t('loading.statusVocab'));
        try { await loadStatusVocab(state.fa.status.col); }
        catch (e) { showError(String(e)); }
        finally { hideLoading(); }
      } else {
        state.statusMode = 'derive';
      }
      state.step = STEP.STATUS; render();
    } finally { hideLoading(); }
    return;
  }

  // Harmonize status → species (TNRS)
  if (state.step === STEP.STATUS) {
    if (state.statusMode === 'column') {
      showLoading(t('loading.statusCol'));
      try {
        const meta = {
          ...buildMetadataPayload(),
          census_years: state.dataFormat === 'wide'
            ? state.widePairs.filter(p => p.year > 0).map(p => p.year)
            : state.censusYears,
        };
        const remaps = buildStatusRemaps();
        if (state.dataFormat === 'wide') {
          await invoke('apply_wide_mapping', {
            request: {
              gfb3_dsn: state.gfb3Dsn,
              identity_exprs: buildFieldExprs().filter(f => !['DBH', 'YR'].includes(f.target_col)),
              dbh_pairs: state.widePairs.filter(p => state.wideDbhCols.includes(p.source_column) && p.year > 0),
              status_remaps: remaps,
              metadata: meta,
            },
          });
        } else {
          await invoke('apply_fields_mapping', {
            request: {
              gfb3_dsn: state.gfb3Dsn,
              fields: buildFieldExprs(),
              dbh_unit: state.dbhUnit,
              status_remaps: remaps,
              metadata: meta,
            },
          });
        }
      } finally { hideLoading(); }
    }
    state.step = STEP.SPECIES;
    render();
    return;
  }

  // Species → apply resolutions → validate
  if (state.step === STEP.SPECIES) {
    const remaps = Object.entries(state.speciesResolutions)
      .filter(([o, r]) => r && r !== o)
      .map(([original, resolved]) => ({ original, resolved }));
    if (remaps.length) {
      showLoading(t('loading.speciesApply'));
      try {
        await invoke('apply_species_resolutions', { request: { remaps } });
      } finally { hideLoading(); }
    }
    showLoading(t('loading.validate'));
    try {
      const result = await invoke('run_validation', { locale: appLocale() });
      state.validationReport = result.validation ?? result;
      state.diagnosticReport = result.diagnostic ?? null;
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
  state.contact.email      = v('f-contact-email') || state.contact.email;
  const ctryVal = v('f-country');
  if (ctryVal) { state.countryName = ctryVal; state.country = isoFromCountry(ctryVal).toUpperCase(); }
  state.submitYear = v('f-submityear') || state.submitYear;
  state.siteName   = v('f-site')  || state.siteName;
  if (state.piSameAsContact) {
    syncPiFromContact();
  } else {
    state.piName  = v('f-pi') || state.piName;
    state.piEmail = v('f-pi-email') || state.piEmail;
  }
  state.curatorName = v('f-curator') || state.curatorName;
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
  const real = new Set(usableCols());

  // PlotID — only real source columns from the loaded file
  const plotCols = (fa.plotId.cols || []).filter(c => real.has(c));
  const prefix   = (fa.plotId.prefix || '').trim();
  if (plotCols.length === 1 && !prefix) {
    fields.push({ kind:'column', target_col:'PlotID', source: plotCols[0] });
  } else if (plotCols.length >= 1 || prefix) {
    fields.push({
      kind: 'concat',
      target_col: 'PlotID',
      sources: [...plotCols],
      sep: '_',
      to_lower: true,
      prefix: prefix || null,
    });
  }

  // TreeID — real columns only. May include PlotID after it is built above.
  const treeCols = (fa.treeId.cols || []).filter(c => real.has(c) || c === 'PlotID');
  if (treeCols.length === 1 && treeCols[0] !== 'PlotID') {
    fields.push({ kind:'column', target_col:'TreeID', source: treeCols[0] });
  } else if (treeCols.length >= 1) {
    fields.push({ kind:'concat', target_col:'TreeID', sources: [...treeCols], sep:'_', to_lower:true, prefix: null });
  }

  // Single-column measurement fields
  if (fa.dbh.col && real.has(fa.dbh.col))
    fields.push({ kind:'column', target_col:'DBH', source: fa.dbh.col });
  if (fa.yr.col && real.has(fa.yr.col))
    fields.push({ kind:'year_from_column', target_col:'YR', source: fa.yr.col });
  if (fa.species.col && real.has(fa.species.col))
    fields.push({ kind:'column', target_col:'Species', source: fa.species.col });

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

  // Status — include whenever chosen in inventory (or column mode override)
  const stCol = state.fa.status.col || (state.statusMode === 'column' ? activeStatusCol() : '');
  if (stCol) fields.push({ kind:'column', target_col:'Status', source: stCol });

  return fields;
}

// ── Export ─────────────────────────────────────────────────────────────────────
async function exportDiagnostic(format) {
  clearError();
  if (!state.diagnosticReport) {
    showError(t('step6.loading'));
    return;
  }
  const isMulti = (state.diagnosticReport.census_type || state.censusType) === 'multi';
  const suffix = isMulti ? '_gfb3_report' : '_gfb2_report';
  const base = (state.diagnosticReport.dataset_name || state.gfb3Dsn || 'dataset').replace(/[^\w\-]+/g, '_');
  const ext = format === 'pdf' ? 'pdf' : 'html';
  const filters = format === 'pdf'
    ? [{ name: t('dialog.filterPdf'), extensions: ['pdf'] }]
    : [{ name: t('dialog.filterHtml'), extensions: ['html'] }];
  const path = await saveDialog({
    defaultPath: `${base}${suffix}.${ext}`,
    filters,
  });
  if (!path) return;
  showLoading(t('loading.exporting'));
  try {
    const saved = await invoke('export_diagnostic_report', {
      request: { path, format, locale: appLocale() },
    });
    clearError();
    const bar = document.querySelector('.diag-export-bar');
    if (bar) {
      let note = document.getElementById('diag-export-note');
      if (!note) {
        note = document.createElement('p');
        note.id = 'diag-export-note';
        note.style.cssText = 'width:100%;margin:.35rem 0 0;font-size:.82rem;color:var(--green-dark)';
        bar.appendChild(note);
      }
      note.textContent = t('step6.exportDone', { path: saved });
    }
  } catch (e) {
    showError(String(e));
  } finally {
    hideLoading();
  }
}

async function doExport() {
  clearError();
  const outDir  = state.exportOutDir || (document.getElementById('f-outdir') ||{}).value||'';
  const base    = (document.getElementById('f-basename')||{}).value||state.gfb3Dsn||'dataset';
  const formats = [...document.querySelectorAll('input[name="fmt"]:checked')].map(e=>e.value);
  if (!outDir.trim()) { showError(t('error.chooseOutDir')); return; }
  if (!formats.length) { showError(t('error.chooseFormat')); return; }
  const constantExpan = (!state.fixedArea && state.expanMode === 'constant' && state.constantExpan !== '')
    ? Number(state.constantExpan)
    : null;
  if (constantExpan != null && (!Number.isFinite(constantExpan) || constantExpan < 0)) {
    showError(t('error.invalidExpan'));
    return;
  }
  showLoading(t('loading.exporting'));
  try {
    const files = await invoke('export', {
      request: {
        output_dir: outDir,
        base_name: base,
        formats,
        keep_alive_only: state.keepAliveOnly,
        fixed_area: state.fixedArea,
        constant_expan: constantExpan,
        curator: state.curatorName.trim(),
        locale: appLocale(),
      },
    });
    document.getElementById('export-result').innerHTML = `
      <div class="finding finding-clean" style="margin-top:1rem">
        <div class="finding-header"><span class="finding-rule">${t('step7.exportComplete')}</span><span class="badge badge-clean">${t('step7.files', { count: files.length })}</span></div>
        <ul style="margin:.4rem 0 0 1rem;font-size:.82rem">${files.map(f=>`<li style="font-family:monospace;margin-bottom:.2rem">${esc(f)}</li>`).join('')}</ul>
      </div>`;
  } catch(e) { showError(String(e)); } finally { hideLoading(); }
}

// ── Leaflet map (centroid picker modal + plot map workspace) ───────────────────
let leafletMap = null, leafletMarker = null, pickedLatLon = null;
let plotLeafletMap = null;
let plotLeafletLayerGroup = null;
let plotLeafletBaseLayers = null;
let plotSymbolLegend = null;

const MAP_SYMBOL_COLORS = [
  '#2d6a4f', '#457b9d', '#e76f51', '#f4a261', '#e9c46a', '#2a9d8f',
  '#264653', '#9b2226', '#bb3e03', '#0a9396', '#6d597a', '#b56576',
  '#355070', '#ee9b00', '#94d2bd', '#005f73', '#ae2012', '#ca6702',
  '#1d3557', '#e56b6f', '#eaac8b', '#40916c', '#52b788', '#6a994e',
];
const MAP_SYMBOL_DEFAULT = '#52b788';
const MAP_SYMBOL_OTHER = '#6c757d';
const MAP_SYMBOL_MAX = 20;

const MAP_CRS_OPTIONS = () => [
  { id: 'EPSG:4326', labelKey: 'crs.epsg4326' },
  { id: 'EPSG:3857', labelKey: 'crs.epsg3857' },
  { id: 'EPSG:4269', labelKey: 'crs.epsg4269' },
  { id: 'EPSG:4214', labelKey: 'crs.epsg4214' },
  { id: 'UTM_N', labelKey: 'crs.utmN' },
  { id: 'UTM_S', labelKey: 'crs.utmS' },
];

function ensureProj4Defs() {
  if (typeof proj4 === 'undefined') return;
  if (!proj4.defs['EPSG:3857']) {
    proj4.defs('EPSG:3857', '+proj=merc +a=6378137 +b=6378137 +lat_ts=0 +lon_0=0 +x_0=0 +y_0=0 +k=1 +units=m +nadgrids=@null +wktext +no_defs +type=crs');
  }
  if (!proj4.defs['EPSG:4269']) {
    proj4.defs('EPSG:4269', '+proj=longlat +datum=NAD83 +no_defs +type=crs');
  }
}

function resolveMapCrsCode() {
  const mv = state.mapView;
  if (mv.crs === 'UTM_N' || mv.crs === 'UTM_S') {
    const z = parseInt(mv.utmZone, 10);
    if (!z || z < 1 || z > 60) return null;
    const epsg = (mv.crs === 'UTM_N' ? 32600 : 32700) + z;
    const code = `EPSG:${epsg}`;
    if (typeof proj4 !== 'undefined' && !proj4.defs[code]) {
      const hemi = mv.crs === 'UTM_N' ? '+north' : '+south';
      proj4.defs(code, `+proj=utm +zone=${z} ${hemi} +datum=WGS84 +units=m +no_defs +type=crs`);
    }
    return code;
  }
  return mv.crs || 'EPSG:4326';
}

function projectToWgs84(yOrLat, xOrLon, crsCode) {
  if (!crsCode || crsCode === 'EPSG:4326') {
    return { lat: yOrLat, lon: xOrLon };
  }
  ensureProj4Defs();
  if (typeof proj4 === 'undefined') {
    throw new Error(t('error.proj4Missing'));
  }
  // proj4 expects [x, y] = [easting/lon, northing/lat]
  const [lon, lat] = proj4(crsCode, 'EPSG:4326', [xOrLon, yOrLat]);
  return { lat, lon };
}

function basenameOf(path) {
  if (!path) return '';
  const trimmed = String(path).replace(/[/\\]+$/, '');
  const i = Math.max(trimmed.lastIndexOf('/'), trimmed.lastIndexOf('\\'));
  return i < 0 ? trimmed : trimmed.slice(i + 1);
}

function stemOf(path) {
  const base = basenameOf(path);
  const dot = base.lastIndexOf('.');
  return dot > 0 ? base.slice(0, dot) : base;
}

function renderConvertView() {
  const cv = state.convertView;
  const fmts = cv.formats;
  const result = cv.result;
  const meta = result
    ? `<p class="convert-meta">${esc(t('convert.meta', {
        rows: Number(result.row_count || 0).toLocaleString(),
        cols: Number(result.column_count || 0).toLocaleString(),
      }))}</p>`
    : '';
  const outputs = result && Array.isArray(result.outputs) && result.outputs.length
    ? `<ul class="convert-outputs">${result.outputs.map(p =>
        `<li><code>${esc(p)}</code></li>`
      ).join('')}</ul>`
    : '';
  return `<div class="convert-view">
    <div>
      <h2>${esc(t('convert.title'))}</h2>
      <p class="step-desc">${esc(t('convert.desc'))}</p>
    </div>
    <div class="convert-panel">
      <div class="convert-field">
        <label>${esc(t('convert.inputLabel'))}</label>
        <div class="convert-row">
          <input type="text" id="convert-input" readonly value="${esc(cv.inputPath)}" placeholder="${esc(t('convert.pickInput'))}" />
          <button type="button" class="btn btn-ghost" id="convert-pick-input">${esc(t('convert.pickInput'))}</button>
        </div>
      </div>
      <div class="convert-field">
        <label>${esc(t('convert.formats'))}</label>
        <div class="convert-formats">
          <label class="convert-chk"><input type="checkbox" id="convert-fmt-csv" ${fmts.csv ? 'checked' : ''} /> ${esc(t('convert.fmtCsv'))}</label>
          <label class="convert-chk"><input type="checkbox" id="convert-fmt-tsv" ${fmts.tsv ? 'checked' : ''} /> ${esc(t('convert.fmtTsv'))}</label>
          <label class="convert-chk"><input type="checkbox" id="convert-fmt-parquet" ${fmts.parquet ? 'checked' : ''} /> ${esc(t('convert.fmtParquet'))}</label>
          <label class="convert-chk"><input type="checkbox" id="convert-fmt-xlsx" ${fmts.xlsx ? 'checked' : ''} /> ${esc(t('convert.fmtXlsx'))}</label>
        </div>
      </div>
      <div class="convert-field">
        <label>${esc(t('convert.outDir'))}</label>
        <div class="convert-row">
          <input type="text" id="convert-outdir" readonly value="${esc(cv.outDir)}" placeholder="${esc(t('convert.phOutDir'))}" />
          <button type="button" class="btn btn-ghost" id="convert-pick-outdir">${esc(t('common.browse'))}</button>
        </div>
      </div>
      <div class="convert-field">
        <label for="convert-basename">${esc(t('convert.baseName'))}</label>
        <input type="text" id="convert-basename" value="${esc(cv.baseName)}" />
      </div>
      <div class="convert-actions">
        <button type="button" class="btn btn-primary" id="convert-run">${esc(t('convert.run'))}</button>
      </div>
      ${meta}
      ${outputs}
    </div>
  </div>`;
}

function mapColOptions(selected) {
  const opts = usableCols().map(c =>
    `<option value="${esc(c)}" ${c === selected ? 'selected' : ''}>${esc(c)}</option>`
  ).join('');
  return `<option value="">${esc(t('common.selectColumn'))}</option>${opts}`;
}

function renderMapView() {
  if (!state.loadResult) {
    return `<div class="plot-map-view">
      <h2>${esc(t('plotMap.title'))}</h2>
      <p class="step-desc">${esc(t('plotMap.desc'))}</p>
      <div class="plot-map-empty">${esc(t('plotMap.needFile'))}</div>
    </div>`;
  }
  const mv = state.mapView;
  const showUtm = mv.crs === 'UTM_N' || mv.crs === 'UTM_S';
  const crsOpts = MAP_CRS_OPTIONS().map(o =>
    `<option value="${o.id}" ${mv.crs === o.id ? 'selected' : ''}>${esc(t(o.labelKey))}</option>`
  ).join('');
  const statusText = mv.status || (mv.latCol && mv.lonCol ? '' : t('plotMap.pickCols'));
  return `<div class="plot-map-view">
    <div>
      <h2>${esc(t('plotMap.title'))}</h2>
      <p class="step-desc">${esc(t('plotMap.desc'))}</p>
    </div>
    <div class="plot-map-toolbar">
      <div class="plot-map-field">
        <label for="map-lat-col">${esc(t('plotMap.lat'))}</label>
        <select id="map-lat-col">${mapColOptions(mv.latCol)}</select>
      </div>
      <div class="plot-map-field">
        <label for="map-lon-col">${esc(t('plotMap.lon'))}</label>
        <select id="map-lon-col">${mapColOptions(mv.lonCol)}</select>
      </div>
      <div class="plot-map-field">
        <label for="map-label-col">${esc(t('plotMap.label'))}</label>
        <select id="map-label-col">${mapColOptions(mv.labelCol)}</select>
      </div>
      <div class="plot-map-field">
        <label for="map-symbol-col">${esc(t('plotMap.symbolBy'))}</label>
        <select id="map-symbol-col">
          <option value="">${esc(t('plotMap.symbolNone'))}</option>
          ${usableCols().map(c =>
            `<option value="${esc(c)}" ${c === mv.symbolCol ? 'selected' : ''}>${esc(c)}</option>`
          ).join('')}
        </select>
      </div>
      <div class="plot-map-field" style="min-width:14rem">
        <label for="map-crs">${esc(t('plotMap.crs'))}</label>
        <select id="map-crs">${crsOpts}</select>
      </div>
      ${showUtm ? `<div class="plot-map-field" style="min-width:6rem">
        <label for="map-utm-zone">${esc(t('plotMap.utmZone'))}</label>
        <input type="number" id="map-utm-zone" min="1" max="60" value="${esc(mv.utmZone || '')}" style="padding:.35rem .45rem;border:1px solid var(--border);border-radius:var(--radius);width:5rem" />
      </div>` : ''}
      <div class="plot-map-actions">
        <button type="button" class="btn btn-primary" id="map-plot-btn">${esc(t('plotMap.plot'))}</button>
        <button type="button" class="btn btn-ghost" id="map-save-html" ${mv.points.length ? '' : 'disabled'}>${esc(t('plotMap.saveHtml'))}</button>
      </div>
      <div class="plot-map-status ${(!mv.latCol || !mv.lonCol) ? 'warn' : ''}" id="map-status">${esc(statusText)}</div>
    </div>
    <p style="font-size:.78rem;color:var(--text-muted);margin:0">${esc(t('plotMap.crsHint'))}</p>
    <div class="plot-map-canvas-wrap">
      <div id="plot-map"></div>
    </div>
  </div>`;
}

function maybeAutoSeedMapCols() {
  const mv = state.mapView;
  if (!state.loadResult) return;
  if (!mv.latCol && state.fa.lat.col) mv.latCol = state.fa.lat.col;
  if (!mv.lonCol && state.fa.lon.col) mv.lonCol = state.fa.lon.col;
  if (!mv.labelCol) {
    if (usableCols().includes('PlotID')) mv.labelCol = 'PlotID';
    else if (state.fa.plotId.cols.length) mv.labelCol = state.fa.plotId.cols[0];
  }
  if (mv.latCol && mv.lonCol) return;
  if (mv.autoTried) return;
  mv.autoTried = true;
  const norm = c => String(c).toLowerCase().replace(/[\s-]+/g, '_');
  const cols = usableCols();
  const find = aliases => cols.find(c => aliases.includes(norm(c)));
  if (!mv.latCol) {
    mv.latCol = find(['lat', 'latitude', 'latitud', 'y_lat', 'coord_y', 'coords_y']) || '';
  }
  if (!mv.lonCol) {
    mv.lonCol = find(['lon', 'long', 'longitude', 'longitud', 'lng', 'x_lon', 'x_long', 'coord_x', 'coords_x']) || '';
  }
  if (!mv.labelCol) {
    mv.labelCol = find(['plotid', 'plot_id', 'plot']) || '';
  }
}

function mapSymbolColorMap(points, symbolCol) {
  const colors = new Map();
  if (!symbolCol) return colors;
  const counts = new Map();
  for (const p of points) {
    const key = (p.symbol == null || String(p.symbol).trim() === '') ? '(blank)' : String(p.symbol);
    counts.set(key, (counts.get(key) || 0) + 1);
  }
  const ranked = [...counts.entries()].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]));
  let i = 0;
  for (const [key] of ranked) {
    if (i < MAP_SYMBOL_MAX) {
      colors.set(key, MAP_SYMBOL_COLORS[i % MAP_SYMBOL_COLORS.length]);
      i += 1;
    } else {
      colors.set(key, MAP_SYMBOL_OTHER);
    }
  }
  return colors;
}

function colorForSymbol(symbol, colorMap, symbolCol) {
  if (!symbolCol || !colorMap.size) return MAP_SYMBOL_DEFAULT;
  const key = (symbol == null || String(symbol).trim() === '') ? '(blank)' : String(symbol);
  return colorMap.get(key) || MAP_SYMBOL_OTHER;
}

function destroyPlotLeaflet() {
  if (plotSymbolLegend && plotLeafletMap) {
    try { plotLeafletMap.removeControl(plotSymbolLegend); } catch (_) {}
    plotSymbolLegend = null;
  }
  if (plotLeafletMap) {
    try { plotLeafletMap.remove(); } catch (_) {}
    plotLeafletMap = null;
    plotLeafletLayerGroup = null;
    plotLeafletBaseLayers = null;
  }
}

function updatePlotSymbolLegend(colorMap, symbolCol) {
  if (!plotLeafletMap) return;
  if (plotSymbolLegend) {
    try { plotLeafletMap.removeControl(plotSymbolLegend); } catch (_) {}
    plotSymbolLegend = null;
  }
  if (!symbolCol || !colorMap.size) return;

  const entries = [...colorMap.entries()];
  const hasOther = [...colorMap.values()].includes(MAP_SYMBOL_OTHER)
    && entries.filter(([, c]) => c === MAP_SYMBOL_OTHER).length > 1;

  plotSymbolLegend = L.control({ position: 'bottomleft' });
  plotSymbolLegend.onAdd = () => {
    const div = L.DomUtil.create('div', 'plot-map-legend');
    L.DomEvent.disableClickPropagation(div);
    L.DomEvent.disableScrollPropagation(div);
    const title = document.createElement('div');
    title.className = 'plot-map-legend-title';
    title.textContent = symbolCol;
    div.appendChild(title);
    let otherAdded = false;
    for (const [label, color] of entries) {
      if (hasOther && color === MAP_SYMBOL_OTHER) {
        if (otherAdded) continue;
        otherAdded = true;
        const row = document.createElement('div');
        row.className = 'plot-map-legend-row';
        row.innerHTML = `<span class="plot-map-legend-swatch" style="background:${MAP_SYMBOL_OTHER}"></span><span>${esc(t('plotMap.symbolOther'))}</span>`;
        div.appendChild(row);
        continue;
      }
      const row = document.createElement('div');
      row.className = 'plot-map-legend-row';
      row.innerHTML = `<span class="plot-map-legend-swatch" style="background:${color}"></span><span>${esc(label)}</span>`;
      div.appendChild(row);
    }
    return div;
  };
  plotSymbolLegend.addTo(plotLeafletMap);
}

function initPlotMapLeaflet() {
  const elMap = document.getElementById('plot-map');
  if (!elMap || typeof L === 'undefined') return;
  destroyPlotLeaflet();
  plotLeafletMap = L.map(elMap, { worldCopyJump: true }).setView([0, 0], 2);

  const street = L.tileLayer('https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png', {
    attribution: '© OpenStreetMap',
    maxZoom: 19,
  });
  const terrain = L.tileLayer('https://{s}.tile.opentopomap.org/{z}/{x}/{y}.png', {
    attribution: '© OpenStreetMap, © OpenTopoMap (CC-BY-SA)',
    maxZoom: 17,
  });
  const satellite = L.tileLayer(
    'https://server.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{z}/{y}/{x}',
    {
      attribution: 'Tiles © Esri',
      maxZoom: 19,
    }
  );
  street.addTo(plotLeafletMap);
  plotLeafletBaseLayers = {
    [t('plotMap.layerStreet')]: street,
    [t('plotMap.layerTerrain')]: terrain,
    [t('plotMap.layerSatellite')]: satellite,
  };
  L.control.layers(plotLeafletBaseLayers, null, { position: 'topright' }).addTo(plotLeafletMap);
  plotLeafletLayerGroup = L.layerGroup().addTo(plotLeafletMap);
  setTimeout(() => {
    plotLeafletMap.invalidateSize();
    if (state.mapView.points.length) drawPlotMapPoints(state.mapView.points);
  }, 60);
}

function drawPlotMapPoints(points) {
  if (!plotLeafletMap || !plotLeafletLayerGroup) return;
  plotLeafletLayerGroup.clearLayers();
  if (!points.length) {
    updatePlotSymbolLegend(new Map(), '');
    return;
  }
  const symbolCol = state.mapView.symbolCol || '';
  const colorMap = mapSymbolColorMap(points, symbolCol);
  const canvasRenderer = L.canvas({ padding: 0.5 });
  const latLngs = [];
  for (const p of points) {
    const ll = [p.lat, p.lon];
    latLngs.push(ll);
    const fill = colorForSymbol(p.symbol, colorMap, symbolCol);
    const m = L.circleMarker(ll, {
      radius: 5,
      color: '#1b4332',
      weight: 1,
      fillColor: fill,
      fillOpacity: 0.9,
      renderer: canvasRenderer,
    });
    const tip = p.plot_id || p.label || `${p.lat.toFixed(5)}, ${p.lon.toFixed(5)}`;
    const tipHtml = symbolCol && p.symbol != null && String(p.symbol).trim() !== ''
      ? `${esc(tip)}<br><span style="opacity:.85">${esc(symbolCol)}: ${esc(String(p.symbol))}</span>`
      : esc(tip);
    m.bindTooltip(tipHtml, { sticky: true, direction: 'top', opacity: 0.95 });
    plotLeafletLayerGroup.addLayer(m);
  }
  updatePlotSymbolLegend(colorMap, symbolCol);
  try {
    plotLeafletMap.fitBounds(L.latLngBounds(latLngs).pad(0.15));
  } catch (_) {}
}

async function loadAndPlotMapPoints() {
  clearError();
  const mv = state.mapView;
  if (!mv.latCol || !mv.lonCol) {
    showError(t('plotMap.pickCols'));
    return;
  }
  const crsCode = resolveMapCrsCode();
  if (!crsCode) {
    showError(t('error.utmZoneInvalid'));
    return;
  }
  showLoading(t('plotMap.loading'));
  try {
    const result = await invoke('get_map_points', {
      latCol: mv.latCol,
      lonCol: mv.lonCol,
      labelCol: mv.labelCol || null,
      symbolCol: mv.symbolCol || null,
      maxPoints: 25000,
    });
    const wgs = [];
    for (const p of result.points) {
      try {
        const c = projectToWgs84(p.lat, p.lon, crsCode);
        if (!Number.isFinite(c.lat) || !Number.isFinite(c.lon)) continue;
        if (Math.abs(c.lat) > 90 || Math.abs(c.lon) > 180) continue;
        wgs.push({
          lat: c.lat,
          lon: c.lon,
          label: p.label || null,
          plot_id: p.plot_id || p.label || null,
          symbol: p.symbol || null,
        });
      } catch (_) { /* skip bad row */ }
    }
    mv.points = wgs;
    mv.truncated = !!result.truncated;
    mv.status = wgs.length
      ? (result.truncated ? t('plotMap.truncated', { n: wgs.length }) : t('plotMap.status', { n: wgs.length }))
      : t('plotMap.noPoints');
    drawPlotMapPoints(wgs);
    const st = document.getElementById('map-status');
    if (st) st.textContent = mv.status;
    el('map-save-html', b => { b.disabled = !wgs.length; });
  } catch (e) {
    showError(String(e));
  } finally {
    hideLoading();
  }
}

function buildStandaloneMapHtml(points) {
  const symbolCol = state.mapView.symbolCol || '';
  const colorMap = mapSymbolColorMap(points, symbolCol);
  const colorObj = {};
  for (const [k, v] of colorMap) colorObj[k] = v;
  const features = points.map(p => ({
    type: 'Feature',
    properties: {
      plot_id: p.plot_id || p.label || '',
      label: p.label || p.plot_id || '',
      symbol: p.symbol == null ? '' : String(p.symbol),
    },
    geometry: { type: 'Point', coordinates: [p.lon, p.lat] },
  }));
  const geojson = JSON.stringify({ type: 'FeatureCollection', features });
  const colorsJson = JSON.stringify(colorObj);
  const symbolColJson = JSON.stringify(symbolCol);
  const defaultColor = JSON.stringify(MAP_SYMBOL_DEFAULT);
  const otherColor = JSON.stringify(MAP_SYMBOL_OTHER);
  const layerStreet = JSON.stringify(t('plotMap.layerStreet'));
  const layerTerrain = JSON.stringify(t('plotMap.layerTerrain'));
  const layerSatellite = JSON.stringify(t('plotMap.layerSatellite'));
  const symbolBlank = JSON.stringify(t('plotMap.symbolBlank'));
  const symbolOtherLabel = JSON.stringify(t('plotMap.symbolOther'));
  const htmlLang = appLocale();
  return `<!DOCTYPE html>
<html lang="${htmlLang}"><head>
<meta charset="UTF-8" />
<title>${esc(t('plotMap.htmlTitle'))}</title>
<link rel="stylesheet" href="https://unpkg.com/leaflet@1.9.4/dist/leaflet.css" />
<script src="https://unpkg.com/leaflet@1.9.4/dist/leaflet.js"><\/script>
<style>
html,body,#map{margin:0;height:100%;} #map{width:100%;}
.plot-map-legend{background:#fff;padding:.55rem .7rem;border-radius:6px;box-shadow:0 1px 4px rgba(0,0,0,.25);font:12px/1.35 system-ui,sans-serif;max-height:40vh;overflow:auto;max-width:220px}
.plot-map-legend-title{font-weight:700;margin-bottom:.35rem}
.plot-map-legend-row{display:flex;align-items:center;gap:.4rem;margin:.18rem 0}
.plot-map-legend-swatch{width:12px;height:12px;border-radius:50%;border:1px solid #1b4332;flex-shrink:0}
</style>
</head><body>
<div id="map"></div>
<script>
const data = ${geojson};
const symbolCol = ${symbolColJson};
const colorMap = ${colorsJson};
const defaultColor = ${defaultColor};
const otherColor = ${otherColor};
function fillFor(sym) {
  if (!symbolCol) return defaultColor;
  const key = (!sym || !String(sym).trim()) ? ${symbolBlank} : String(sym);
  return colorMap[key] || otherColor;
}
const map = L.map('map');
const street = L.tileLayer('https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png', { attribution: '© OpenStreetMap', maxZoom: 19 });
const terrain = L.tileLayer('https://{s}.tile.opentopomap.org/{z}/{x}/{y}.png', { attribution: '© OpenTopoMap', maxZoom: 17 });
const satellite = L.tileLayer('https://server.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{z}/{y}/{x}', { attribution: '© Esri', maxZoom: 19 });
street.addTo(map);
L.control.layers({ [${layerStreet}]: street, [${layerTerrain}]: terrain, [${layerSatellite}]: satellite }).addTo(map);
const layer = L.geoJSON(data, {
  pointToLayer: (f, latlng) => L.circleMarker(latlng, {
    radius: 5, color: '#1b4332', weight: 1,
    fillColor: fillFor(f.properties && f.properties.symbol),
    fillOpacity: 0.9
  }),
  onEachFeature: (f, l) => {
    const text = (f.properties && (f.properties.plot_id || f.properties.label)) || '';
    if (text) l.bindTooltip(String(text), { sticky: true, direction: 'top', opacity: 0.95 });
  }
}).addTo(map);
if (symbolCol && Object.keys(colorMap).length) {
  const legend = L.control({ position: 'bottomleft' });
  legend.onAdd = function () {
    const div = L.DomUtil.create('div', 'plot-map-legend');
    div.innerHTML = '<div class="plot-map-legend-title"></div>';
    div.querySelector('.plot-map-legend-title').textContent = symbolCol;
    let otherAdded = false;
    Object.keys(colorMap).forEach(function (label) {
      const color = colorMap[label];
      let text = label;
      if (color === otherColor) {
        if (otherAdded) return;
        otherAdded = true;
        text = ${symbolOtherLabel};
      }
      const row = document.createElement('div');
      row.className = 'plot-map-legend-row';
      row.innerHTML = '<span class="plot-map-legend-swatch" style="background:' + color + '"></span><span></span>';
      row.lastChild.textContent = text;
      div.appendChild(row);
    });
    return div;
  };
  legend.addTo(map);
}
try { map.fitBounds(layer.getBounds().pad(0.15)); } catch (e) { map.setView([0,0], 2); }
<\/script>
</body></html>`;
}

function exportBaseName() {
  const fromField = (document.getElementById('f-basename') || {}).value;
  const raw = (fromField || state.gfb3Dsn || computeDsn() || 'dataset').trim();
  return raw.replace(/[^\w.\-]+/g, '_') || 'dataset';
}

async function savePlotMapHtml() {
  clearError();
  if (!state.mapView.points.length) return;
  const base = exportBaseName();
  const path = await saveDialog({
    defaultPath: `${base}_map.html`,
    filters: [{ name: t('dialog.filterHtml'), extensions: ['html'] }],
  });
  if (!path) return;
  showLoading(t('plotMap.saving'));
  try {
    await invoke('save_text_file', {
      path,
      contents: buildStandaloneMapHtml(state.mapView.points),
    });
    state.mapView.status = t('plotMap.saved', { path });
    const st = document.getElementById('map-status');
    if (st) st.textContent = state.mapView.status;
  } catch (e) {
    showError(String(e));
  } finally {
    hideLoading();
  }
}

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

// Workspace tabs live in the header (not re-rendered) — bind once
document.querySelectorAll('.workspace-tab').forEach(btn => {
  btn.addEventListener('click', () => {
    const view = btn.dataset.view;
    if (!view || view === state.workspaceView) return;
    if (view !== 'map') destroyPlotLeaflet();
    state.workspaceView = view;
    clearError();
    render();
  });
});

try {
  render();
  console.log('✓ Initial render complete');
} catch(e) {
  console.error('Initial render failed:', e);
  document.getElementById('main').innerHTML = `<div style="color:red;padding:2rem;font-family:monospace;white-space:pre-wrap">${esc(t('error.init', { msg: e.message, stack: e.stack }))}</div>`;
}
