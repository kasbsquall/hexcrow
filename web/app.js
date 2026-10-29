// Hexcrow demo. Renders the recorded Esmeralda run (data/run.json) and checks every public fact
// against the public indexer: current epoch, invoice status in the book, each escrow output, and
// each transaction's verdict.
(() => {
  'use strict';

  const STATUS = ['Issued', 'Funded', 'Approved', 'Settled', 'Refunded'];
  const STAMP = { Issued: 'ISSUED', Funded: 'FUNDED', Approved: 'APPROVED', Settled: 'SETTLED', Refunded: 'REFUNDED' };
  const STATUS_ICON = { Issued: 'file-plus', Funded: 'lock-key', Approved: 'check-circle', Settled: 'flag-checkered', Refunded: 'arrow-u-up-left' };
  const COPY = { public: 'original', client: 'canario', contractor: 'rosa', mediator: 'salvia', accountant: 'azul' };
  const VIEWS = Object.keys(COPY);
  const VIEW_NAME = { public: 'Public', client: 'Client', contractor: 'Contractor', mediator: 'Mediator', accountant: 'Accountant' };
  const AMOUNT_LABEL = { client: 'Amount', contractor: 'Amount owed', mediator: 'Amount, shown to the mediator', accountant: 'Amount, read with the escrow view key' };
  const ARCHIVE = 'https://github.com/kasbsquall/hexcrow/blob/main/evidence/indexer-results/';
  const FETCH_TIMEOUT_MS = 9000;
  const REFRESH_MS = 60000;
  const REPLAY_STEP_MS = 320;
  const reduce = matchMedia('(prefers-reduced-motion: reduce)');

  // The attacks of the run, as a visitor would try them. `guard` names what refused it.
  const TRIES = {
    'contractor-sets-lock': { title: 'Fund it with a code you already know', by: 'Signed by the contractor', guard: ['book-open', "The book: only the client's key funds"] },
    'stranger-claim': { title: 'Steal it as a stranger', by: 'Signed by a key that is on no invoice', guard: ['key', "Contractor's key"] },
    'claim-without-code': { title: 'Claim without the approval code', by: 'Signed by the contractor', guard: ['lock-key', 'Hash lock'] },
    'early-refund': { title: 'Take the money back early', by: 'Signed by the client', guard: ['hourglass-medium', 'Time lock'] },
    'self-approve': { title: 'Approve your own invoice', by: 'Signed by the contractor', guard: ['book-open', "The book: only the client's key approves"] },
    'mediator-alone': { title: 'Pay yourself as the mediator', by: 'Signed by the mediator alone', guard: ['key', "Contractor's key"] },
    'ruling-without-mediator': { title: 'Rule without the mediator', by: 'Signed by the contractor alone', guard: ['scales', "Mediator's key"] },
    'refund-after-approval': { title: 'Approve, then take it back', by: 'Signed by the client', guard: ['hourglass-medium', 'Time lock'] },
    'stranger-claim:after-approval': { title: 'Claim with the code once it is public', by: 'Signed by a stranger', guard: ['key', "Contractor's key"] },
    'late-ruling': { title: 'Rule after the window has closed', by: 'Signed by the mediator and the contractor', guard: ['hourglass-medium', 'Time lock'] },
  };
  const ACTOR_ICON = { stranger: 'user-circle-dashed', contractor: 'pen-nib', client: 'buildings', mediator: 'scales' };

  const $ = (id) => document.getElementById(id);
  const icon = (n) => `<i class="ph-light ph-${n}" aria-hidden="true"></i>`;
  const esc = (s) => String(s).replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
  const short = (h, a = 8, b = 6) => (h.length > a + b + 1 ? `${h.slice(0, a)}…${b ? h.slice(-b) : ''}` : h);
  const pad4 = (n) => String(n).padStart(4, '0');
  const num = (n) => n.toLocaleString('en-US');
  const ep = (n) => String(n); // epochs are identifiers, printed without separators
  const tari = (micro) => (micro / 1e6).toLocaleString('en-US', { maximumFractionDigits: 6 });
  const sk = (label) => `<span class="sk" role="status" aria-label="${esc(label)}"></span>`;
  const sha256hex = async (text) => [...new Uint8Array(await crypto.subtle.digest('SHA-256', new TextEncoder().encode(text)))]
    .map((b) => b.toString(16).padStart(2, '0')).join('');

  // ---- state -------------------------------------------------------------------------------
  const state = {
    run: null,
    view: 'public',
    invoice: 1,
    epoch: null,            // number once read, 'off' if the indexer is unreachable
    book: null,             // Map(no -> {status, deadline, claimUntil, hash, code, ruling}) once read, 'off' if unreachable
    escrows: new Map(),     // invoice no -> {state: 'unspent'|'spent'|'off', root}
    verdicts: new Map(),    // tx id -> {state: 'wait'|'ok'|'neutral'|'bad'|'off', text, archived}
    tried: new Map(),       // tx id -> verdict shown in "Try to steal it"
  };

  // ---- sound -------------------------------------------------------------------------------
  const sound = (() => {
    let muted = false;
    try { muted = localStorage.getItem('hexcrow-muted') === '1'; } catch (_) { /* storage blocked */ }
    const clips = {};
    for (const name of ['switch', 'stamp', 'verified']) {
      const a = new Audio(`sounds/${name}.ogg`);
      a.preload = 'auto';
      a.volume = 0.28;
      clips[name] = a;
    }
    const btn = $('mute');
    // A toggle named "Sound" that reads pressed while the sound is on.
    const paint = () => {
      btn.setAttribute('aria-pressed', String(!muted));
      btn.innerHTML = `${icon(muted ? 'speaker-simple-slash' : 'speaker-simple-high')}<span class="lbl">Sound</span>`;
    };
    btn.addEventListener('click', () => {
      muted = !muted;
      try { localStorage.setItem('hexcrow-muted', muted ? '1' : '0'); } catch (_) { /* storage blocked */ }
      paint();
    });
    paint();
    return {
      // Only ever called from a user gesture.
      play(name) {
        if (muted) return;
        const a = clips[name].cloneNode();
        a.volume = 0.28;
        a.play().catch(() => {});
      },
    };
  })();

  // ---- indexer -----------------------------------------------------------------------------
  async function getJSON(path) {
    const ctl = new AbortController();
    const t = setTimeout(() => ctl.abort(), FETCH_TIMEOUT_MS);
    try {
      const r = await fetch(state.run.indexer + path, { signal: ctl.signal, cache: 'no-store' });
      if (!r.ok) throw Object.assign(new Error(`HTTP ${r.status}`), { status: r.status });
      return await r.json();
    } finally {
      clearTimeout(t);
    }
  }

  const hexOf = (v) => (v && v['@cbor'] === 'bytes' ? v.hex : null);

  // The book's state is a CBOR map: invoice no ->
  // [hash, contractor, client, deadline, claim_until, lock, root, code, [status, []], mediator, ruling].
  function decodeBook(json) {
    const entries = json.state.up.substate.Component.body.state[0].entries;
    const out = new Map();
    for (const [no, f] of entries) {
      out.set(no, {
        hash: hexOf(f[0]),
        deadline: f[3],
        claimUntil: f[4],
        root: hexOf(f[6]) || (typeof f[6] === 'string' ? f[6] : null),
        code: Array.isArray(f[7]) ? String.fromCharCode(...f[7]) : null,
        status: STATUS[f[8][0]],
        ruling: hexOf(f[10]),
      });
    }
    return out;
  }
  const bookEntry = (i) => (state.book instanceof Map ? state.book.get(i.no) || null : null);

  // The escrow output as a substate: up while unspent (with the condition root it is locked by),
  // down once a leaf spent it. Unlike transaction results, the indexer does not prune this.
  const UTXO_PREFIX = 'utxo_' + '01'.repeat(32) + '_';
  async function readEscrow(i) {
    try {
      const json = await getJSON('substates/' + UTXO_PREFIX + i.escrow_commitment);
      const auth = json.state.up.substate.Utxo.output.auth;
      return { state: 'unspent', root: auth && auth.Script };
    } catch (e) {
      return { state: e.status === 404 ? 'spent' : 'off' };
    }
  }

  // A verdict as the indexer finalized it, compared with what the run recorded.
  function readVerdict(json, row) {
    // After a while the indexer keeps only this short form for a transaction the chain rejected.
    const rejected = json.result && json.result.Rejected;
    if (rejected) return judge({ final_decision: 'Reject', result: 'Reject', finalized_time: rejected.rejected_time }, row, false);
    const fin = json.result && json.result.Finalized;
    if (!fin) return { state: 'bad', text: 'Not finalized on the indexer' };
    const res = fin.execution_result && fin.execution_result.finalize && fin.execution_result.finalize.result;
    return judge({ final_decision: fin.final_decision, result: res ? Object.keys(res)[0] : '', finalized_time: fin.finalized_time }, row, false);
  }

  function judge(v, row, archived) {
    const committed = v.final_decision === 'Commit' && v.result === 'Accept';
    const where = archived ? 'result archived' : 'confirmed live on the indexer';
    if (committed !== row.accepted) return { state: 'bad', text: `Disagrees with the recording (${where})`, archived };
    if (committed) return { state: 'ok', text: `Committed, ${where}`, archived };
    if (v.result === 'AcceptFeeRejectRest') return { state: 'neutral', text: `Refused, fee kept, ${where}`, archived };
    return { state: 'neutral', text: `Refused, nothing committed, ${where}`, archived };
  }

  // The indexer prunes transaction results within a few epochs, whatever its /info advertises.
  // A 404 then falls back to the raw result saved from the same indexer right after the run
  // (evidence/indexer-results/<tx>.json, snapshotted by tools/snapshot_verdicts.py), linked so it can be opened.
  function fromSnapshot(row) {
    const snap = state.snapshot && state.snapshot.verdicts[row.tx];
    if (!snap) return { state: 'off', text: 'Dropped by the indexer, not archived' };
    return judge(snap, row, true);
  }

  async function verdictOf(row) {
    try {
      return readVerdict(await getJSON(`transactions/${row.tx}/result`), row);
    } catch (e) {
      // Dropped by the indexer, or the indexer is down: the archived result still answers, and says so.
      if (e.status === 404 || (state.snapshot && state.snapshot.verdicts[row.tx])) return fromSnapshot(row);
      return { state: 'off', text: 'Indexer unreachable' };
    }
  }

  // The three reads run together; the epoch shows as soon as it arrives.
  async function refreshNetwork() {
    const epoch = getJSON('network').then((j) => { state.epoch = j.epoch; }, () => { state.epoch = 'off'; }).then(paintNet);
    const book = getJSON('substates/' + state.run.book).then((j) => { state.book = decodeBook(j); }, () => { state.book = 'off'; });
    const escrows = Promise.all(state.run.invoices.map(async (i) => state.escrows.set(i.no, await readEscrow(i))));
    await Promise.all([epoch, book, escrows]);
    render();
  }

  async function verifyAll(byUser) {
    const rows = state.run.invoices.flatMap((inv) => inv.timeline);
    rows.forEach((r) => state.verdicts.set(r.tx, { state: 'wait', text: 'Checking on chain' }));
    renderStubs();
    await Promise.all(rows.map(async (r) => state.verdicts.set(r.tx, await verdictOf(r))));
    renderStubs();
    const all = [...state.verdicts.values()];
    if (byUser && all.every((v) => v.state === 'ok' || v.state === 'neutral')) sound.play('verified');
  }

  // ---- derived facts -----------------------------------------------------------------------
  const inv = () => state.run.invoices.find((i) => i.no === state.invoice);
  const done = (i, kind) => i.timeline.find((t) => t.kind === kind && t.accepted);
  const tx = (i, kind) => {
    const r = done(i, kind);
    return r ? r.tx : null;
  };
  const ruling = (i) => done(i, 'rule');

  function recordedStatus(i) {
    if (tx(i, 'refund')) return 'Refunded';
    const r = ruling(i);
    if (r) return r.for === 'contractor' ? 'Settled' : 'Refunded';
    if (tx(i, 'claim')) return 'Settled';
    if (tx(i, 'approve')) return 'Approved';
    return tx(i, 'fund') ? 'Funded' : 'Issued';
  }

  function liveStatus(i) {
    if (state.book === null) return { status: null, source: 'wait' };
    if (state.book === 'off' || !state.book.has(i.no)) return { status: recordedStatus(i), source: 'recorded' };
    return { status: state.book.get(i.no).status, source: 'live' };
  }

  const epochKnown = () => typeof state.epoch === 'number';

  // How the money left the escrow, if it did: which of the four ways out, in plain words.
  function exitOf(i) {
    if (tx(i, 'claim')) return { leaf: 'claim', words: 'the claim, on the client’s approval' };
    const r = ruling(i);
    if (r) return { leaf: r.for === 'contractor' ? 'ruling-contractor' : 'ruling-client', words: `the mediator’s ruling for the ${r.for}` };
    if (tx(i, 'refund')) return { leaf: 'refund', words: 'the refund, after the claim window closed' };
    return null;
  }
  const paidOut = (exit) => exit.leaf === 'claim' || exit.leaf === 'ruling-contractor';
  // A ruling the mediator and the contractor signed after the window closed, which the chain refused.
  const lateRuling = (i) => i.timeline.some((t) => t.attack === 'late-ruling');

  // ---- rendering: shell --------------------------------------------------------------------
  function paintNet() {
    const pulse = $('pulse');
    if (epochKnown()) {
      pulse.dataset.state = 'on';
      // One beat per successful read of the indexer, so the dot means "just refreshed".
      if (!reduce.matches) pulse.animate([{ transform: 'scale(1.8)', opacity: .4 }, { transform: 'scale(1)', opacity: 1 }], { duration: 280, easing: 'cubic-bezier(.23,1,.32,1)' });
      $('net-text').innerHTML = `<span class="hide-sm">Esmeralda testnet, epoch</span><span class="show-sm">Epoch</span> <b>${ep(state.epoch)}</b>`;
    } else if (state.epoch === 'off') {
      pulse.dataset.state = 'off';
      $('net-text').textContent = 'Indexer unreachable: showing the recorded run';
    }
  }

  function renderProof() {
    const attacks = state.run.invoices.flatMap((i) => i.timeline.filter((t) => t.kind === 'attack'));
    const through = attacks.filter((t) => t.accepted).length;
    $('pr-attacks').textContent = String(attacks.length);
    $('pr-attacks-note').textContent = through ? `${through} got through.` : 'The chain refused every one.';
    $('to-tries-l').textContent = `Replay the ${attacks.length} attacks yourself`;
    $('pr-invoices').textContent = String(state.run.invoices.length);
    if (state.book === null) return;
    const statuses = state.run.invoices.map((i) => liveStatus(i).status);
    const of = `<small>of ${state.run.invoices.length}</small>`;
    $('pr-paid').innerHTML = `${statuses.filter((s) => s === 'Settled').length}${of}`;
    $('pr-returned').innerHTML = `${statuses.filter((s) => s === 'Refunded').length}${of}`;
    $('pr-public').innerHTML = `0${of}`;
  }

  // ---- rendering: the form -----------------------------------------------------------------
  const smudge = () => `<svg class="smudge" preserveAspectRatio="none" aria-hidden="true"><rect width="100%" height="100%" filter="url(#smudge)"/></svg>`;
  const notOnCopy = (what) => `${smudge()}<span class="notcopy">${icon('lock-simple')}Not on this copy<span class="sr-only">: ${what} is hidden from this holder</span></span>`;
  const stampSVG = (word, w) => `<svg viewBox="0 0 ${w} 60" aria-hidden="true"><g filter="url(#ink)" fill="none" stroke="currentColor"><rect x="3" y="3" width="${w - 6}" height="54" rx="3" stroke-width="3.2"/><rect x="9" y="9" width="${w - 18}" height="42" rx="1.5" stroke-width="1.3"/><text x="${w / 2}" y="41" text-anchor="middle" fill="currentColor" stroke="none" font-family="Figtree, sans-serif" font-weight="700" font-size="27" letter-spacing="3">${word}</text></g></svg>`;
  const archiveLink = (id, label) => `<a href="${ARCHIVE}${id}.json" title="The raw result the indexer served right after the run">${label}</a>`;
  // The indexer drops transaction results within a few epochs: a transaction links to the indexer only
  // while the indexer still serves its result, and to the archived copy otherwise.
  const txLink = (id, label) => {
    const v = state.verdicts.get(id);
    const live = v && !v.archived && (v.state === 'ok' || v.state === 'neutral' || v.state === 'bad');
    return live ? `<a href="${state.run.indexer}transactions/${id}/result" title="Open the raw result on the indexer">${label || short(id, 16, 0)}</a>`
      : archiveLink(id, label || short(id, 16, 0));
  };

  // A wax seal on the escrow while it holds the money; broken once a leaf spent it.
  const sealSVG = (open) => `<svg class="seal${open ? ' open' : ''}" viewBox="0 0 56 56" aria-hidden="true">
      <path class="wax" d="M28 3.5c3.4 0 4.6 2.6 7.6 3.5 3 .9 5.6-.7 8 1.6 2.4 2.3 1 5 1.9 8 .9 3 3.5 4.2 3.5 7.6s-2.6 4.6-3.5 7.6c-.9 3 .7 5.6-1.6 8-2.3 2.4-5 1-8 1.9-3 .9-4.2 3.5-7.6 3.5s-4.6-2.6-7.6-3.5c-3-.9-5.6.7-8-1.6-2.4-2.3-1-5-1.9-8C9.9 35.1 7.3 33.9 7.3 30.5s2.6-4.6 3.5-7.6c.9-3-.7-5.6 1.6-8 2.3-2.4 5-1 8-1.9 3-.9 4.2-3.5 7.6-3.5Z" transform="translate(0 -2.5)"/>
      <circle class="rim" cx="28" cy="28" r="15"/>
      <path class="emboss" d="M28 17.5 37 22.7v10.6L28 38.5 19 33.3V22.7Z"/><path class="emboss" d="M24 26.5h8M24 29.5h5.5"/>
      ${open ? '<path class="crack" d="M14 12 25 25l-4 6 13 13"/>' : ''}
    </svg>`;

  function statusBox(i) {
    const { status, source } = liveStatus(i);
    const val = status ? `${icon(STATUS_ICON[status])}${status}` : sk('Reading the status from the book');
    const r = ruling(i);
    const by = r && status && (status === 'Settled' || status === 'Refunded')
      ? `<p class="by">${icon('scales')}By the mediator’s ruling</p>` : '';
    const note = source === 'live'
      ? '<p class="live" data-state="live"><span class="dot"></span>Read live from the book</p>'
      : source === 'recorded' ? '<p class="live" data-state="recorded"><span class="dot"></span>Recorded status, indexer unreachable</p>' : '';
    return `<div class="box s2 b-status"><p class="lab">${icon('flag')}Status</p><p class="val">${val}</p>${by}${note}</div>`;
  }

  // The first thing read on the form: what this copy is, in a sentence.
  function captionText(i, view) {
    const { status } = liveStatus(i);
    const s = status || recordedStatus(i);
    const amt = `<b>${tari(i.parties_only.amount_micro_tari)} tTARI</b>`;
    const r = ruling(i);
    const exit = exitOf(i);
    const outcome = {
      Settled: r ? 'paid to the contractor on the mediator’s ruling' : 'approved and paid to the contractor',
      Refunded: r ? 'returned to the client on the mediator’s ruling' : 'returned to the client after the claim window closed',
      Approved: 'approved',
      Funded: 'funded and is waiting for approval',
      Issued: 'issued and not funded yet',
    }[s];
    if (view === 'public') {
      return ['globe-simple', `Anyone with the chain can read that invoice ${pad4(i.no)} exists, when it is due, and that it was ${outcome}. <b>The amount, the work and who the two sides are did not transfer to this copy.</b>`];
    }
    const late = lateRuling(i);
    if (view === 'client') {
      const tail = !exit ? 'It holds the approval code; nothing moves until it approves, the mediator rules, or the window closes.'
        : exit.leaf === 'claim' ? 'It chose the approval code, and its approval is what released the money.'
        : exit.leaf === 'ruling-contractor' ? 'It never answered, so the mediator both sides chose ruled for the contractor.'
        : exit.leaf === 'ruling-client' ? 'The contractor quit the work, and the mediator returned the money without waiting for the window to close.'
        : late ? 'It never approved, and a ruling sent after the window closed was refused, so the money came back to it.'
        : 'Nobody approved or called the mediator, so the money came back once the claim window closed.';
      return ['buildings', `The client sees what it locked, ${amt}, and for what work. ${tail}`];
    }
    if (view === 'contractor') {
      const tail = !exit ? 'Payment still needs the client’s approval or the mediator’s ruling.'
        : exit.leaf === 'claim' ? 'It was paid when the client approved.'
        : exit.leaf === 'ruling-contractor' ? 'The client went silent, and the mediator’s ruling paid it.'
        : exit.leaf === 'ruling-client' ? 'It quit the work, and the mediator returned the money to the client.'
        : late ? 'The client never approved; the ruling it signed with the mediator came after the window closed, the chain refused it, and the money went back to the client.'
        : 'Nobody approved, so the money went back to the client.';
      return ['pen-nib', `The contractor sees what it is owed, ${amt}, and for what. ${tail}`];
    }
    if (view === 'mediator') {
      if (r) {
        const caller = r.for === 'contractor' ? 'contractor' : 'client';
        return ['scales', `The ${caller} called the mediator in and showed it the invoice. <b>The mediator ruled for the ${r.for}.</b> Its written ruling stays off chain; the book keeps only its hash${i.ruling && i.ruling.salt_lost ? '' : ', checked below'}.`];
      }
      if (late) {
        return ['scales', 'The mediator and the contractor signed a ruling after the claim window had closed. <b>The chain refused it,</b> and the money went back to the client.'];
      }
      return ['scales', `Nobody called the mediator in, so <b>its copy stayed blank.</b> Its key can act only together with the client or the contractor, and only before epoch ${ep(i.claim_until)}.`];
    }
    return ['calculator', 'Given the invoice text and its salt, the accountant recomputes the hash stored in the book and reads the amount with the invoice’s view key. <b>It can check this invoice and move nothing.</b>'];
  }

  function captionBox(i, view) {
    const [ic, text] = captionText(i, view);
    return `<div class="box s6 b-caption">${icon(ic)}<p>${text}</p></div>`;
  }

  const seesPrivate = (i, view) => view === 'client' || view === 'contractor' || view === 'accountant' || (view === 'mediator' && Boolean(ruling(i)));

  function privateBoxes(i, view) {
    const p = i.parties_only;
    if (!seesPrivate(i, view)) {
      return `<div class="box s4 b-text hidden"><p class="lab">${icon('file-text')}Invoice text</p>${notOnCopy('the invoice text')}</div>
        <div class="box s2 b-amount hidden"><p class="lab">${icon('coins')}Amount</p>${smudge()}<p class="ghostfig" aria-hidden="true"><span class="ghostnum"></span><span class="unit">tTARI</span></p><span class="notcopy">${icon('lock-simple')}Not on this copy<span class="sr-only">: the amount is hidden from this holder</span></span></div>`;
    }
    const given = view === 'accountant' ? `<p class="given">${icon('hand-arrow-down')}Given to the accountant with its salt</p>`
      : view === 'mediator' ? `<p class="given">${icon('hand-arrow-down')}Shown to the mediator by the ${ruling(i).for} who called it in</p>` : '';
    return `<div class="box s4 b-text"><p class="lab">${icon('file-text')}Invoice text</p><p class="val">${esc(p.text)}</p>${given}</div>
      <div class="box s2 b-amount"><p class="lab">${icon('coins')}${AMOUNT_LABEL[view]}</p><p class="amount"><span class="big">${tari(p.amount_micro_tari)}</span><span class="unit">tTARI</span></p></div>`;
  }

  // The two sides chose each other and the mediator; only the public cannot tell who they are.
  function partyBox(role, key, view, i) {
    const names = { client: ['buildings', 'Client'], contractor: ['pen-nib', 'Contractor'], mediator: ['scales', 'Mediator'] };
    const [ic, name] = names[role];
    const r = ruling(i);
    const knows = view === 'client' || view === 'contractor' || (view === 'mediator' && r && r.for === role);
    const who = view === role ? `<p class="who">${icon('user-circle')}This is you</p>`
      : knows ? `<p class="who">${icon('handshake')}Known to you off chain; only this key is on chain</p>`
      : `<p class="who away">${icon('eye-slash')}Who it is: not on this copy</p>`;
    return `<div class="box s2 party"><p class="lab">${icon(ic)}${name}</p>${who}<p class="key mono-ish" title="${key}">One-time key ${short(key, 6, 6)}</p></div>`;
  }

  function deadlineBox(i) {
    const { status } = liveStatus(i);
    let now;
    if (status === 'Settled' || status === 'Refunded') {
      const exit = exitOf(i);
      now = exit ? `Closed through ${exit.words}` : 'Closed';
    } else if (status === 'Approved') {
      now = `Approved; the contractor can claim before epoch ${ep(i.claim_until)}`;
    } else if (epochKnown()) {
      const left = i.deadline_epoch - state.epoch;
      now = `Current epoch ${ep(state.epoch)}, ${left > 0 ? `${left} ${left === 1 ? 'epoch' : 'epochs'} left to approve` : 'approval deadline passed'}`;
    } else {
      now = state.epoch === 'off' ? 'Current epoch unknown' : sk('Reading the current epoch');
    }
    return `<div class="box s3 b-deadline"><p class="lab">${icon('hourglass-medium')}Deadlines, in epochs of about 70 min</p><p class="val">Approve before epoch ${ep(i.deadline_epoch)}<br>Claim or rule before epoch ${ep(i.claim_until)}</p><p class="small">${now}</p></div>`;
  }

  // While the escrow read is on its way the seal follows the recording; the words wait for the chain.
  function escrowBox(i) {
    const e = state.escrows.get(i.no);
    const exit = exitOf(i);
    const where = exit ? (paidOut(exit) ? 'paid to the contractor' : 'returned to the client') : '';
    let val;
    let line = '';
    let open = Boolean(exit);
    if (!e) {
      val = sk('Reading the escrow output');
    } else if (e.state === 'unspent') {
      const fromBook = bookEntry(i) && bookEntry(i).root;
      open = false;
      val = 'Sealed: holds the money';
      line = e.root === (fromBook || i.escrow_root) ? 'Locked by exactly these rules, with no key that can open it. Checked live.'
        : `${icon('warning')}Locked by a different root than ${fromBook ? 'the book records' : 'the recorded run'}`;
    } else if (e.state === 'spent') {
      open = true;
      val = exit ? `Opened: ${where}` : 'Opened';
      line = `${exit ? `Through ${exit.words}. ` : ''}Checked live: the output is gone from the chain.`;
    } else {
      val = exit ? 'Opened, as recorded' : 'Sealed, as recorded';
      line = `${icon('cloud-slash')}Indexer unreachable`;
    }
    return `<div class="box s3 b-escrow"><p class="lab">${icon('vault')}Escrow</p>${sealSVG(open)}<div><p class="val">${val}</p>${line ? `<p class="small">${line}</p>` : ''}</div></div>`;
  }

  function idsBox(i) {
    const live = bookEntry(i);
    const publicHash = live ? live.hash : i.invoice_hash;
    const rows = [
      ['book-open', 'Book', state.run.book],
      ['stack', 'Template', state.run.template],
      ['hash', `Invoice hash${live ? ', read from the book' : ''}`, publicHash],
      ['cube', 'Escrow output', i.escrow_commitment],
      ['tree-structure', 'Condition root', i.escrow_root],
      ['buildings', 'Client key', i.client_key],
      ['pen-nib', 'Contractor key', i.contractor_key],
      ['scales', 'Mediator key', i.mediator_key],
    ];
    if (i.ruling) rows.push(['scroll', 'Ruling hash', i.ruling.ruling_hash]);
    return `<details class="box s6 ids"><summary>${icon('caret-right')}On-chain ids<span class="hint">book, template, hashes, escrow output, one-time keys</span></summary>
      <dl>${rows.filter((r) => r[2]).map(([ic, k, v]) => `<dt>${icon(ic)}${k}</dt><dd>${esc(v)}</dd>`).join('')}</dl></details>`;
  }

  // The accountant's check and the mediator's check, computed in the browser.
  function checkBox(i, view) {
    if (view === 'accountant') {
      return `<div class="box s6"><p class="lab">${icon('seal-check')}The accountant’s check</p><dl class="check" id="acct-check">
        <dt>Salt</dt><dd>${esc(i.parties_only.salt)}</dd>
        <dt>sha256(salt|text)</dt><dd id="acct-hash">${sk('Computing the hash in your browser')}</dd>
        <dt>Result</dt><dd id="acct-result">${sk('Comparing with the book')}</dd>
        <dt>Memo, read with the view key</dt><dd>${esc(i.parties_only.memo)}</dd>
      </dl></div>`;
    }
    if (view === 'mediator' && i.ruling && i.ruling.salt_lost) {
      return `<div class="box s6"><p class="lab">${icon('warning')}This ruling’s hash cannot be opened</p><p class="val">Its salt was lost: a second copy of our run script started the same ruling and overwrote the salt before it was saved. The hash in the book stands, and the money moved as it says, but nobody can show the text behind it. Invoice ${pad4(3)} shows the full check.</p></div>`;
    }
    if (view === 'mediator' && i.ruling && i.parties_only.ruling_text) {
      return `<div class="box s6"><p class="lab">${icon('scroll')}The mediator’s ruling, kept off chain</p><p class="val ruling">${esc(i.parties_only.ruling_text)}</p>
        <dl class="check">
          <dt>sha256(salt|ruling)</dt><dd id="rule-hash">${sk('Computing the hash in your browser')}</dd>
          <dt>Result</dt><dd id="rule-result">${sk('Comparing with the book')}</dd>
        </dl></div>`;
    }
    return '';
  }

  // The nodes are taken before the hash is computed, so a slow hash of a previous sheet writes nowhere.
  async function runChecks(i) {
    const live = bookEntry(i);
    const p = i.parties_only;
    const acct = $('acct-hash');
    if (acct) {
      const res = $('acct-result');
      const hex = await sha256hex(`${p.salt}|${p.text}`);
      const target = live ? live.hash : i.invoice_hash;
      if (!acct.isConnected) return;
      acct.textContent = hex;
      res.innerHTML = hex === target
        ? `<span class="match">${icon('seal-check')}Matches the ${live ? 'hash in the book, computed in your browser' : 'recorded invoice hash'}</span>`
        : `<span class="match no">${icon('warning')}Does not match the invoice hash</span>`;
    }
    const rule = $('rule-hash');
    if (rule) {
      const res = $('rule-result');
      const hex = await sha256hex(`${p.ruling_salt}|${p.ruling_text}`);
      const target = live && live.ruling ? live.ruling : i.ruling.ruling_hash;
      if (!rule.isConnected) return;
      rule.textContent = hex;
      res.innerHTML = hex === target
        ? `<span class="match">${icon('seal-check')}Matches the ruling hash ${live && live.ruling ? 'in the book, computed in your browser' : 'recorded for this invoice'}</span>`
        : `<span class="match no">${icon('warning')}Does not match the ruling hash</span>`;
    }
  }

  function approvalLine(i, view) {
    const live = bookEntry(i);
    const code = (live && live.code) || i.approval_code;
    if (code) {
      return `<p class="code">${icon('key')}<span>Approval code, public in the book since approval: <b>${esc(code)}</b>. On its own it cannot move the money: the claim also needs the contractor’s key.</span></p>`;
    }
    if (view === 'client' && !exitOf(i)) return `<p class="code">${icon('key')}<span>You hold the approval code. It stays off chain until you approve.</span></p>`;
    return '';
  }

  function act(iconName, label, st, txId) {
    const isDone = Boolean(txId);
    return `<span class="act${isDone || st.startsWith('Possible') ? '' : ' off'}">${icon(iconName)}<span>${label}</span><span class="st">${st}</span>${isDone ? `<span class="tx">${txLink(txId, short(txId, 8, 0))}</span>` : ''}</span>`;
  }

  function clientActions(i, status) {
    const passed = epochKnown() && state.epoch >= i.deadline_epoch;
    const closed = epochKnown() && state.epoch >= i.claim_until;
    const exit = exitOf(i);
    const approve = tx(i, 'approve') ? 'Done' : exit || passed ? 'No longer possible' : `Possible before epoch ${ep(i.deadline_epoch)}`;
    const refund = tx(i, 'refund') ? 'Done'
      : exit ? 'No longer possible: already closed'
      : status === 'Approved' ? 'No longer possible: approved'
      : closed ? 'Possible now' : `From epoch ${ep(i.claim_until)}`;
    const r = ruling(i);
    return act('lock-key', 'Fund', tx(i, 'fund') ? 'Done' : 'Next', tx(i, 'fund'))
      + act('check-circle', 'Approve', approve, tx(i, 'approve'))
      + act('arrow-u-up-left', 'Refund', refund, tx(i, 'refund'))
      + (r && r.for === 'client' ? act('scales', 'Co-sign the ruling', 'Done', r.tx) : '');
  }

  function contractorActions(i, status) {
    const exit = exitOf(i);
    const r = ruling(i);
    const claim = tx(i, 'claim') ? 'Done'
      : exit ? 'No longer possible: already closed'
      : status === 'Approved' ? `Possible now, before epoch ${ep(i.claim_until)}` : 'After approval';
    return act('hand-coins', 'Claim', claim, tx(i, 'claim'))
      + (r && r.for === 'contractor' ? act('scales', 'Co-sign the ruling', 'Done', r.tx) : '');
  }

  // `hexcrow verify`, run by the contractor before starting work, from public data only. Shown as the
  // CLI printed it (cli/src/main.rs, `verify`), at the epoch it ran, so its first person and its epoch read as recorded.
  function contractorCheck(i) {
    const c = i.contractor_check;
    if (!c) return '';
    const ok = c.funds_locked_as_agreed ?? c.safe_to_start_work;
    const at = (c.checks.map((k) => /epoch (\d+)/.exec(k.check)).find(Boolean) || [])[1];
    const lines = [
      `<span class="cmd">$ hexcrow verify ${i.no} --expect-tari ${tari(i.parties_only.amount_micro_tari)}</span>`,
      `invoice ${i.no}: approve before epoch ${ep(i.deadline_epoch)}, claim before epoch ${ep(i.claim_until)}`,
      ...c.checks.map((k) => `  <span class="${k.ok ? 'ok' : 'fail'}">[${k.ok ? 'ok' : 'FAIL'}]</span> ${esc(k.check)}`),
      ok ? 'funds locked as agreed; payment still needs the client\'s approval' : 'do not start work',
    ];
    return `<div class="precheck"><p class="lab">${icon('terminal-window')}Run by the contractor before starting work${at ? `, at epoch ${ep(Number(at))}` : ''}</p><pre class="cli">${lines.join('\n')}</pre></div>`;
  }

  function accessBox(i, view, status) {
    const s = status || recordedStatus(i);
    if (view === 'public') {
      return `<h2>${icon('globe-simple')}This copy has no actions</h2><p class="lead">This is what anyone with the chain reads: one-time keys, the deadlines, the status and the hashes under “On-chain ids”. No amount, no names, no invoice text.</p>${approvalLine(i, view)}`;
    }
    if (view === 'client') return `<h2>${icon('buildings')}Client actions</h2>${approvalLine(i, view)}<div class="actions">${clientActions(i, s)}</div>`;
    if (view === 'contractor') return `<h2>${icon('pen-nib')}Contractor actions</h2>${approvalLine(i, view)}<div class="actions">${contractorActions(i, s)}</div>${contractorCheck(i)}`;
    if (view === 'mediator') {
      const r = ruling(i);
      const closed = exitOf(i) || (epochKnown() && state.epoch >= i.claim_until);
      const did = r ? act('scales', `Ruled for the ${r.for}`, 'Done', r.tx)
        : closed ? act('scales', 'Rule', 'No longer possible: already closed', null)
        : act('scales', 'Rule', 'Only if called in, with that side’s key', null);
      return `<h2>${icon('scales')}Mediator actions</h2><p class="lead">The mediator cannot pay itself, cannot act alone, and loses its say when the claim window closes.</p><div class="actions">${did}</div>`;
    }
    return `<h2>${icon('calculator')}Accountant: reads, cannot spend</h2><p class="lead">No key can take the money out of the escrow: it opens only under its four rules. The view key reads the amount; it cannot move it.</p>`;
  }

  function rulesBox(i, view, status) {
    const s = status || recordedStatus(i);
    const exit = exitOf(i);
    const openPath = !exit && (s === 'Funded' || s === 'Approved');
    const yours = (who) => (view === who && openPath ? '<span class="yours">Your path</span>' : '');
    const used = (leaf) => (exit && exit.leaf === leaf ? ' used' : '');
    return `<div class="box s6 rules">
      <h2>${icon('scroll')}The rules</h2>
      <p class="fine">Four ways out of the escrow, public and enforced by the chain itself.</p>
      <div class="rule-rows">
        <div class="rule-row${used('claim')}">${icon('hand-coins')}<b>Claim ${yours('contractor')}</b><span>Approval code + contractor key, before epoch ${ep(i.claim_until)}</span></div>
        <div class="rule-row${used('refund')}">${icon('arrow-u-up-left')}<b>Refund ${yours('client')}</b><span>Client key, from epoch ${ep(i.claim_until)}</span></div>
        <div class="rule-row${used('ruling-contractor')}">${icon('scales')}<b>Ruling for the contractor ${yours('mediator')}</b><span>Mediator key + contractor key, before epoch ${ep(i.claim_until)}</span></div>
        <div class="rule-row${used('ruling-client')}">${icon('scales')}<b>Ruling for the client</b><span>Mediator key + client key, before epoch ${ep(i.claim_until)}</span></div>
      </div>
    </div>`;
  }

  function sheetHTML(i, view) {
    const { status } = liveStatus(i);
    const word = STAMP[status || recordedStatus(i)];
    return `
      <div class="stamp" data-word="${word}">${stampSVG(word, word.length > 7 ? 196 : 178)}</div>
      <div class="grid">
        <div class="box s3 b-title"><p class="ttl">Hexcrow invoice</p><p class="small">${VIEW_NAME[view]} copy</p></div>
        <div class="box s1 b-no"><p class="lab">No.</p><p class="num">${pad4(i.no)}</p></div>
        ${statusBox(i)}
        ${captionBox(i, view)}
        ${privateBoxes(i, view)}
        ${partyBox('client', i.client_key, view, i)}
        ${partyBox('contractor', i.contractor_key, view, i)}
        ${partyBox('mediator', i.mediator_key, view, i)}
        ${deadlineBox(i)}
        ${escrowBox(i)}
        ${checkBox(i, view)}
        <div class="box s6 access">${accessBox(i, view, status)}</div>
        ${rulesBox(i, view, status)}
        ${idsBox(i)}
      </div>`;
  }

  function renderVersus(i) {
    const amount = `${tari(i.parties_only.amount_micro_tari)} tTARI`;
    const rows = [
      ['coins', 'Amount', `${amount}, in the contract balance`, 'Hidden in a commitment. The parties and the holders of the view key read it.'],
      ['buildings', 'Who paid', 'The client account, on the deposit', `A one-time key, ${short(i.client_key, 8, 0)}`],
      ['pen-nib', 'Who got paid', 'The contractor account, on the release', `A one-time key, ${short(i.contractor_key, 8, 0)}`],
      ['scales', 'Who settles a dispute', 'An arbiter address, on every ruling it signs', 'A one-time mediator key, and only the hash of its ruling'],
      ['link', 'Other payments of the same people', 'Linkable through the whole history of both accounts', 'Not through keys: each invoice has its own keys for every party. Timing can still hint.'],
      ['scroll', 'The rules and the status', 'Public', 'Public'],
    ];
    $('vs-body').innerHTML = rows.map(([ic, fact, open, ours]) =>
      `<tr><th scope="row">${icon(ic)}${fact}</th><td class="open${open === ours ? ' same' : ''}">${esc(open)}</td><td>${esc(ours)}</td></tr>`).join('');
  }

  function renderNextCopy() {
    const n = VIEWS.indexOf(state.view);
    const next = VIEWS[(n + 1) % VIEWS.length];
    $('next-copy').innerHTML = `<span>Copy ${n + 1} of ${VIEWS.length}</span><button type="button" id="next-btn">${icon('arrow-right')}Next copy: ${VIEW_NAME[next]}</button>`;
    $('next-btn').addEventListener('click', () => setView(next));
  }

  // ---- rendering: aside --------------------------------------------------------------------
  const STORY = {
    claim: 'Approved, then claimed',
    'ruling-contractor': 'Client went silent; the mediator paid the contractor',
    'ruling-client': 'Contractor quit; the mediator returned it',
    refund: 'No approval in time; refunded after the window',
  };

  function renderInvoices() {
    $('invs').innerHTML = state.run.invoices.map((i) => {
      const { status } = liveStatus(i);
      const st = status ? `${status === 'Funded' ? `<span class="spin">${icon('hourglass-simple')}</span>` : icon(STATUS_ICON[status])}${status}` : sk('Reading the status');
      const exit = exitOf(i);
      const story = exit ? STORY[exit.leaf] : `Approve before epoch ${ep(i.deadline_epoch)}`;
      return `<li><button class="inv" type="button" data-no="${i.no}" aria-current="${i.no === state.invoice}">
        <span class="nm">Invoice ${pad4(i.no)}</span><span class="st">${st}</span>
        <span class="meta">${story} · ${i.timeline.length} transactions</span></button></li>`;
    }).join('');
  }

  function stubLabel(row, view, i) {
    if (row.kind === 'fund') {
      return seesPrivate(i, view) ? `Funded by the client, ${tari(i.parties_only.amount_micro_tari)} tTARI locked` : 'Funded by the client, amount hidden';
    }
    return row.kind === 'attack' ? `Attack: ${row.label[0].toLowerCase()}${row.label.slice(1)}` : row.label;
  }

  function verdictHTML(row, v) {
    const ic = { ok: 'seal-check', neutral: 'seal-check', bad: 'warning', off: 'cloud-slash', wait: 'circle-notch' }[v.state];
    const text = v.archived ? v.text.replace('result archived', archiveLink(row.tx, 'result archived')) : esc(v.text);
    return `<span class="ver" data-state="${v.state}">${icon(ic)}<span>${text}</span></span>`;
  }

  // Who signed an attack, as the try names it ("the mediator and the contractor"), not only its first signer.
  const signedBy = (r) => {
    const t = TRIES[r.attack];
    return t ? t.by[0].toLowerCase() + t.by.slice(1) : `signed by the ${r.actor}`;
  };

  function renderStubs() {
    const i = inv();
    const rows = i.timeline;
    const rejected = rows.filter((r) => !r.accepted).length;
    const keep = document.activeElement && document.activeElement.id === 'recheck';
    $('tl-h').textContent = `Invoice ${pad4(i.no)} on chain`;
    $('tl-count').innerHTML = `${rows.length} transactions: ${rows.length - rejected} committed, ${rejected} refused. <button class="recheck" id="recheck" type="button">${icon('arrows-clockwise')}Check again on the indexer</button>`;
    $('tl-foot').textContent = 'Fees in µT, 1 tTARI = 1,000,000 µT. The indexer drops results after a few hours; each one was archived as served.';
    if (keep) $('recheck').focus({ preventScroll: true });
    // The entrance plays once per invoice; live refreshes repaint without replaying it.
    $('stubs').classList.toggle('settled', state.paintedInvoice === i.no);
    state.paintedInvoice = i.no;
    $('stubs').innerHTML = rows.map((r, n) => {
      const v = state.verdicts.get(r.tx) || { state: 'wait', text: 'Checking on chain' };
      const fee = r.fee_micro_tari ? `<span>Fee ${num(r.fee_micro_tari)} µT</span>` : '<span>No fee</span>';
      return `<li class="stub ${r.accepted ? 'ok' : 'rej'}" style="--i:${Math.min(n, 7)}">
        <span class="i">${n + 1}</span>
        <p class="what">${esc(stubLabel(r, state.view, i))}${r.kind === 'attack' ? ` <span class="who">${esc(signedBy(r))}</span>` : ''}</p>
        ${r.why ? `<p class="why">${esc(r.why)}</p>` : ''}
        <p class="meta">${txLink(r.tx)}${fee}</p>
        <p class="meta">${verdictHTML(r, v)}</p>
        <span class="mini">${stampSVG(r.accepted ? 'COMMITTED' : 'REFUSED', r.accepted ? 196 : 164)}</span>
      </li>`;
    }).join('');
    $('recheck').addEventListener('click', () => verifyAll(true));
  }

  // ---- Try to steal it ----------------------------------------------------------------------
  function attackRows() {
    return state.run.invoices.flatMap((i) => i.timeline.filter((t) => t.kind === 'attack').map((t) => ({ ...t, invoice: i.no })));
  }

  function renderTries() {
    const rows = attackRows();
    $('tally-of').textContent = String(rows.length);
    $('tries').innerHTML = rows.map((r, n) => {
      const t = TRIES[r.attack] || { title: r.label, by: `Signed by the ${r.actor}`, guard: ['shield', 'The chain'] };
      return `<li class="try" data-tx="${r.tx}" data-state="idle" style="--i:${Math.min(n, 7)}">
        <span class="who" title="${esc(r.actor)}">${icon(ACTOR_ICON[r.actor] || 'user')}</span>
        <p class="what">${esc(t.title)}<small>${esc(t.by)}, invoice ${pad4(r.invoice)}</small></p>
        <p class="guard">${icon(t.guard[0])}${esc(t.guard[1])}</p>
        <button class="go" type="button">${icon('lightning')}<span class="lbl">Try it</span><span class="sr-only">: ${esc(t.title)}</span></button>
        <p class="out" aria-live="polite"></p>
        <span class="refused" aria-hidden="true"></span>
      </li>`;
    }).join('');
  }

  // The headline number is what the run recorded; the line under it is what the replays found just now.
  function paintTally() {
    const rows = attackRows();
    $('tally-n').textContent = String(rows.filter((r) => !r.accepted).length);
    $('tally-of').textContent = `of ${rows.length} refused on Esmeralda`;
    const tried = rows.map((r) => state.tried.get(r.tx)).filter(Boolean);
    const refused = tried.filter((v) => v.state === 'neutral').length;
    const through = tried.filter((v) => v.state === 'ok' || v.state === 'bad').length;
    const off = tried.filter((v) => v.state === 'off').length;
    const line = $('tally-through');
    if (!tried.length) {
      line.textContent = 'Replay them to ask the indexer again';
      line.dataset.state = '';
    } else {
      const parts = [refused === tried.length ? `${tried.length === 1 ? 'refused' : 'all refused'} again` : `${refused} refused again`];
      if (through) parts.push(`${through} got through`);
      if (off) parts.push(`${off} not checked`);
      line.textContent = `Replayed ${tried.length} of ${rows.length}: ${parts.join(', ')}`;
      line.dataset.state = through ? 'through' : refused ? 'refused' : '';
    }
    const all = tried.length === rows.length && !off;
    $('try-all').setAttribute('aria-disabled', String(all));
  }

  // The verdict shown is the one the indexer (or the archive) gives now, not the recording's.
  async function tryOne(li) {
    if (li.dataset.state === 'wait' || (li.dataset.state === 'done' && li.dataset.verdict !== 'off')) return;
    const row = attackRows().find((r) => r.tx === li.dataset.tx);
    const btn = li.querySelector('.go');
    const hadFocus = document.activeElement === btn;
    li.dataset.state = 'wait';
    btn.setAttribute('aria-disabled', 'true');
    const out = li.querySelector('.out');
    out.innerHTML = `<span class="ver">${icon('circle-notch')}Asking the indexer for the verdict</span>`;
    const v = await verdictOf(row);
    state.verdicts.set(row.tx, v);
    state.tried.set(row.tx, v);
    const [st, ic, head] = v.state === 'neutral' ? ['refused', 'shield-check', 'Refused by the chain']
      : v.state === 'off' ? ['off', 'cloud-slash', `Not checked: ${v.text}`]
      : ['through', 'warning', v.text];
    const source = v.archived ? archiveLink(row.tx, 'result archived from the indexer')
      : st === 'off' ? '' : txLink(row.tx, 'confirmed live on the indexer');
    const fee = st === 'refused' ? (row.fee_micro_tari ? `fee ${num(row.fee_micro_tari)} µT kept` : 'nothing committed') : '';
    const reason = st === 'refused' && row.why ? row.why.replace(/^(Rejected|Refused) by the (chain|book): /, '') : '';
    out.innerHTML = `<span class="ver" data-state="${st}">${icon(ic)}${esc(head)}</span>${reason ? `<span>${esc(reason)}</span>` : ''}${source || fee ? `<span>${[source, fee].filter(Boolean).join(', ')}</span>` : ''}`;
    li.dataset.verdict = st;
    li.dataset.state = 'done';
    if (st === 'off') {
      btn.removeAttribute('aria-disabled');
      btn.querySelector('.lbl').textContent = 'Try again';
    } else {
      const word = st === 'refused' ? 'REFUSED' : 'THROUGH';
      const stamp = li.querySelector('.refused');
      stamp.dataset.word = word;
      stamp.innerHTML = stampSVG(word, 164);
      sound.play('stamp');
      // The button gives way to the stamp: keyboard focus moves on to the next attack, or to the tally.
      if (hadFocus) {
        const next = document.querySelector('.try[data-state="idle"] .go');
        (next || $('tally')).focus({ preventScroll: Boolean(next) });
      }
    }
    paintTally();
  }

  async function replayAll() {
    if ($('try-all').getAttribute('aria-disabled') === 'true') return;
    $('try-all').setAttribute('aria-disabled', 'true');
    const rows = [...document.querySelectorAll('.try')].filter((li) => li.dataset.state === 'idle' || li.dataset.verdict === 'off');
    for (const li of rows) {
      tryOne(li);
      await new Promise((r) => setTimeout(r, reduce.matches ? 0 : REPLAY_STEP_MS));
    }
  }

  // ---- render + transitions ----------------------------------------------------------------
  // A re-render rewrites buttons; whoever had focus gets it back on the button that replaced it.
  function focusKey() {
    const a = document.activeElement;
    if (!a || a === document.body) return null;
    if (a.classList.contains('inv')) return `.inv[data-no="${a.dataset.no}"]`;
    if (a.id === 'next-btn' || a.id === 'recheck') return `#${a.id}`;
    return $('sheet').contains(a) ? '#sheet' : null;
  }

  function render() {
    const i = inv();
    const keep = focusKey();
    document.documentElement.dataset.view = state.view;
    const sheet = $('sheet');
    const openIds = sheet.querySelector('.ids[open]') !== null;
    sheet.innerHTML = sheetHTML(i, state.view);
    if (openIds) sheet.querySelector('.ids').open = true;
    sheet.setAttribute('aria-labelledby', 'tab-' + state.view);
    renderProof();
    renderInvoices();
    renderStubs();
    renderVersus(i);
    renderNextCopy();
    runChecks(i);
    const back = keep && document.querySelector(keep);
    if (back && back !== document.activeElement) back.focus({ preventScroll: true });
  }

  // The crow hops when a copy changes hands.
  function hop() {
    if (reduce.matches) return;
    try {
      $('perch').animate([{ translate: '0 0' }, { translate: '0 -9px' }, { translate: '0 0' }],
        { duration: 280, easing: 'cubic-bezier(.23,1,.32,1)', composite: 'add' });
    } catch (_) { /* individual transform properties unsupported: no hop */ }
  }

  function writeURL() {
    const q = new URLSearchParams({ invoice: String(state.invoice), view: state.view });
    history.replaceState(null, '', `?${q}`);
  }

  function slideSheet(prevCopy) {
    if (reduce.matches) return null;
    const sheet = $('sheet');
    const wrap = sheet.parentElement;
    wrap.querySelectorAll('.ghost').forEach((g) => g.remove());
    const ghost = sheet.cloneNode(true);
    ['id', 'role', 'tabindex', 'aria-labelledby'].forEach((a) => ghost.removeAttribute(a));
    ghost.querySelectorAll('[id]').forEach((n) => n.removeAttribute('id'));
    ghost.setAttribute('aria-hidden', 'true');
    ghost.classList.add('ghost');
    ghost.style.setProperty('--copy', `var(--${prevCopy})`);
    wrap.appendChild(ghost);
    ghost.animate([{ transform: 'translateY(0)', opacity: 1 }, { transform: 'translateY(56px) rotate(.8deg)', opacity: 0 }],
      { duration: 240, easing: 'cubic-bezier(.77,0,.175,1)', fill: 'forwards' }).onfinish = () => ghost.remove();
    return () => {
      $('sheet').animate([{ transform: 'translateY(18px)' }, { transform: 'translateY(0)' }], { duration: 240, easing: 'cubic-bezier(.23,1,.32,1)' });
      const stamp = $('sheet').querySelector('.stamp');
      stamp.animate([{ transform: 'rotate(-8deg) scale(1.04)', opacity: 0 }, { transform: 'rotate(-8deg) scale(.95)', opacity: 1, offset: .6 }, { transform: 'rotate(-8deg) scale(1)', opacity: 1 }],
        { duration: 220, delay: 140, easing: 'cubic-bezier(.23,1,.32,1)', fill: 'backwards' });
    };
  }

  function paintTabs() {
    document.querySelectorAll('.tab').forEach((t) => {
      const on = t.dataset.v === state.view;
      t.setAttribute('aria-selected', String(on));
      t.tabIndex = on ? 0 : -1;
    });
  }

  function setView(view) {
    if (view === state.view) return;
    const after = slideSheet(COPY[state.view]);
    state.view = view;
    paintTabs();
    render();
    if (after) after();
    hop();
    writeURL();
    sound.play('switch');
  }

  function setInvoice(no) {
    if (no === state.invoice) return;
    const after = slideSheet(COPY[state.view]);
    state.invoice = no;
    render();
    if (after) after();
    writeURL();
    sound.play('stamp');
  }

  function bind() {
    const tabs = [...document.querySelectorAll('.tab')];
    tabs.forEach((t, n) => {
      t.addEventListener('click', () => setView(t.dataset.v));
      t.addEventListener('keydown', (e) => {
        const moves = { ArrowRight: (n + 1) % tabs.length, ArrowLeft: (n - 1 + tabs.length) % tabs.length, Home: 0, End: tabs.length - 1 };
        if (!(e.key in moves)) return;
        e.preventDefault();
        tabs[moves[e.key]].focus();
        setView(tabs[moves[e.key]].dataset.v);
      });
    });
    $('invs').addEventListener('click', (e) => {
      const b = e.target.closest('.inv');
      if (b) setInvoice(Number(b.dataset.no));
    });
    $('tries').addEventListener('click', (e) => {
      const b = e.target.closest('.go');
      if (b) tryOne(b.closest('.try'));
    });
    $('try-all').addEventListener('click', replayAll);
    // From "What ran": go to the attacks and replay them, so the verdicts land as the list arrives.
    $('to-tries').addEventListener('click', (e) => {
      e.preventDefault();
      const h = $('hex-h');
      h.scrollIntoView({ behavior: reduce.matches ? 'auto' : 'smooth', block: 'start' });
      h.focus({ preventScroll: true });
      setTimeout(replayAll, reduce.matches ? 0 : 500);
    });
  }

  function readURL() {
    const q = new URLSearchParams(location.search);
    const v = q.get('view');
    const n = Number(q.get('invoice'));
    if (VIEWS.includes(v)) state.view = v;
    if (state.run.invoices.some((i) => i.no === n)) state.invoice = n;
    paintTabs();
  }

  async function start() {
    try {
      const r = await fetch('data/run.json', { cache: 'no-store' });
      if (!r.ok) throw new Error(`HTTP ${r.status}`);
      state.run = await r.json();
    } catch (err) {
      $('sheet').innerHTML = `<div class="box s6"><p class="val">${icon('warning')} The recorded run could not be loaded. Reload the page to try again.</p></div>`;
      return;
    }
    try {
      const s = await fetch('data/verdicts.json', { cache: 'no-store' });
      if (s.ok) state.snapshot = await s.json();
    } catch (_) {
      // Without a snapshot, pruned verdicts show as such; live reads are unaffected.
    }
    $('f-tests').textContent = `${state.run.engine_tests_passed} engine tests pass (local run of the compiled template)`;
    readURL();
    bind();
    renderTries();
    paintTally();
    render();
    refreshNetwork();
    verifyAll(false);
    setInterval(refreshNetwork, REFRESH_MS);
  }

  start();
})();
