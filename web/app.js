// Hexcrow demo. Renders the recorded Esmeralda run (data/run.json) and checks every public fact
// against the public indexer: current epoch, invoice status in the book, and each transaction's verdict.
(() => {
  'use strict';

  const STATUS = ['Issued', 'Funded', 'Approved', 'Settled', 'Refunded'];
  const STAMP = { Issued: 'ISSUED', Funded: 'FUNDED', Approved: 'APPROVED', Settled: 'SETTLED', Refunded: 'REFUNDED' };
  const STATUS_ICON = { Issued: 'file-plus', Funded: 'lock-key', Approved: 'check-circle', Settled: 'flag-checkered', Refunded: 'arrow-counter-clockwise' };
  const COPY = { public: 'original', client: 'canario', contractor: 'rosa', accountant: 'azul' };
  const VIEWS = Object.keys(COPY);
  const SEES_PRIVATE = { public: false, client: true, contractor: true, accountant: true };
  const AMOUNT_LABEL = { client: 'Amount', contractor: 'Amount owed', accountant: 'Amount, read with the escrow view key' };
  const FETCH_TIMEOUT_MS = 9000;
  const REFRESH_MS = 60000;
  const TOUR_START_MS = 1600;
  const TOUR_STEP_MS = 1700;
  const reduce = matchMedia('(prefers-reduced-motion: reduce)');

  const $ = (id) => document.getElementById(id);
  const icon = (n) => `<i class="ph-light ph-${n}" aria-hidden="true"></i>`;
  const esc = (s) => String(s).replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
  const short = (h, a = 8, b = 6) => (h.length > a + b + 1 ? `${h.slice(0, a)}…${b ? h.slice(-b) : ''}` : h);
  const pad4 = (n) => String(n).padStart(4, '0');
  const num = (n) => n.toLocaleString('en-US');
  const ep = (n) => String(n); // epochs are identifiers, printed without separators
  const tari = (micro) => (micro / 1e6).toLocaleString('en-US', { maximumFractionDigits: 6 });
  const sk = (label) => `<span class="sk" role="status" aria-label="${esc(label)}"></span>`;

  // ---- state -------------------------------------------------------------------------------
  const state = {
    run: null,
    view: 'public',
    invoice: 1,
    epoch: null,            // number once read, 'off' if the indexer is unreachable
    book: null,             // Map(no -> {status, deadline, hash, code}) once read, 'off' if unreachable
    verdicts: new Map(),    // tx id -> {state: 'wait'|'ok'|'bad'|'off', text}
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
    const paint = () => {
      btn.setAttribute('aria-pressed', String(muted));
      btn.innerHTML = `${icon(muted ? 'speaker-simple-slash' : 'speaker-simple-high')}<span class="lbl">${muted ? 'Sound off' : 'Sound on'}</span>`;
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
        const a = clips[name];
        a.currentTime = 0;
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
      if (!r.ok) throw new Error(`HTTP ${r.status}`);
      return await r.json();
    } finally {
      clearTimeout(t);
    }
  }

  const hexOf = (v) => (v && v['@cbor'] === 'bytes' ? v.hex : null);

  // The book's state is a CBOR map: invoice no -> [hash, contractor, client, lock, deadline, root, code, [status, []]].
  function decodeBook(json) {
    const entries = json.state.up.substate.Component.body.state[0].entries;
    const out = new Map();
    for (const [no, f] of entries) {
      out.set(no, {
        hash: hexOf(f[0]),
        deadline: f[4],
        code: Array.isArray(f[6]) ? String.fromCharCode(...f[6]) : null,
        status: STATUS[f[7][0]],
      });
    }
    return out;
  }

  // A verdict as the indexer finalized it, compared with what the run recorded.
  function readVerdict(json, row) {
    const fin = json.result && json.result.Finalized;
    if (!fin) return { state: 'bad', text: 'Not finalized on the indexer' };
    const res = fin.execution_result && fin.execution_result.finalize && fin.execution_result.finalize.result;
    const kind = res ? Object.keys(res)[0] : '';
    const committed = fin.final_decision === 'Commit' && kind === 'Accept';
    const when = fin.finalized_time ? `, ${fin.finalized_time.slice(0, 16)} UTC` : '';
    if (committed !== row.accepted) return { state: 'bad', text: 'Indexer disagrees with the recording' };
    if (committed) return { state: 'ok', text: `On chain: committed${when}` };
    if (kind === 'AcceptFeeRejectRest') return { state: 'ok', text: `On chain: rejected, fee charged${when}` };
    return { state: 'ok', text: 'On chain: rejected, nothing committed' };
  }

  async function refreshNetwork() {
    try {
      state.epoch = (await getJSON('network')).epoch;
    } catch (_) {
      state.epoch = 'off';
    }
    try {
      state.book = decodeBook(await getJSON('substates/' + state.run.book));
    } catch (_) {
      state.book = 'off';
    }
    paintNet();
    render();
  }

  async function verifyAll(byUser) {
    const rows = state.run.invoices.flatMap((inv) => inv.timeline);
    rows.forEach((r) => state.verdicts.set(r.tx, { state: 'wait', text: 'Checking on chain' }));
    renderStubs();
    await Promise.all(rows.map(async (r) => {
      try {
        state.verdicts.set(r.tx, readVerdict(await getJSON(`transactions/${r.tx}/result`), r));
      } catch (_) {
        state.verdicts.set(r.tx, { state: 'off', text: 'Indexer unreachable' });
      }
    }));
    renderStubs();
    const all = [...state.verdicts.values()];
    if (byUser && all.every((v) => v.state === 'ok')) sound.play('verified');
  }

  // ---- derived facts -----------------------------------------------------------------------
  const inv = () => state.run.invoices.find((i) => i.no === state.invoice);
  const tx = (i, kind) => {
    const r = i.timeline.find((t) => t.kind === kind && t.accepted);
    return r ? r.tx : null;
  };

  function recordedStatus(i) {
    const order = ['refund', 'claim', 'approve', 'fund', 'issue'];
    const names = { refund: 'Refunded', claim: 'Settled', approve: 'Approved', fund: 'Funded', issue: 'Issued' };
    const k = order.find((s) => tx(i, s));
    return names[k] || 'Issued';
  }

  function liveStatus(i) {
    if (state.book === null) return { status: null, source: 'wait' };
    if (state.book === 'off' || !state.book.has(i.no)) return { status: recordedStatus(i), source: 'recorded' };
    return { status: state.book.get(i.no).status, source: 'live' };
  }

  const epochKnown = () => typeof state.epoch === 'number';

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

  // ---- rendering: the form -----------------------------------------------------------------
  const smudge = () => `<svg class="smudge" preserveAspectRatio="none" aria-hidden="true"><rect width="100%" height="100%" filter="url(#smudge)"/></svg>`;
  const notOnCopy = (what) => `${smudge()}<span class="notcopy">${icon('lock-simple')}Not on this copy<span class="sr-only">: ${what} is hidden from this holder</span></span>`;
  const stampSVG = (word, w) => `<svg viewBox="0 0 ${w} 60" aria-hidden="true"><g filter="url(#ink)" fill="none" stroke="currentColor"><rect x="3" y="3" width="${w - 6}" height="54" rx="3" stroke-width="3.2"/><rect x="9" y="9" width="${w - 18}" height="42" rx="1.5" stroke-width="1.3"/><text x="${w / 2}" y="41" text-anchor="middle" fill="currentColor" stroke="none" font-family="Figtree, sans-serif" font-weight="700" font-size="27" letter-spacing="3">${word}</text></g></svg>`;
  const txLink = (id, label) => `<a href="${state.run.indexer}transactions/${id}/result" title="Open the raw result on the indexer">${label || short(id, 16, 0)}</a>`;

  function statusBox(i) {
    const { status, source } = liveStatus(i);
    const val = status ? `${icon(STATUS_ICON[status])}${status}` : sk('Reading the status from the book');
    const note = source === 'live'
      ? '<p class="live" data-state="live"><span class="dot"></span>Read live from the book</p>'
      : source === 'recorded' ? '<p class="live" data-state="recorded"><span class="dot"></span>Recorded status, indexer unreachable</p>' : '';
    return `<div class="box s2 b-status"><p class="lab">${icon('flag')}Status</p><p class="val">${val}</p>${note}</div>`;
  }

  function deadlineBox(i) {
    let now;
    if (epochKnown()) {
      const left = i.deadline_epoch - state.epoch;
      now = `Current epoch ${ep(state.epoch)}, ${left > 0 ? `${left} ${left === 1 ? 'epoch' : 'epochs'} left` : 'deadline passed'}`;
    } else {
      now = state.epoch === 'off' ? 'Current epoch unknown' : sk('Reading the current epoch');
    }
    return `<div class="box s2 b-deadline"><p class="lab">${icon('hourglass-medium')}Deadline</p><p class="val">Epoch ${ep(i.deadline_epoch)}</p><p class="small">${now}</p></div>`;
  }

  function privateBoxes(i, view) {
    const p = i.parties_only;
    if (!SEES_PRIVATE[view]) {
      return `<div class="box s4 b-text hidden"><p class="lab">${icon('file-text')}Invoice text</p>${notOnCopy('the invoice text')}</div>
        <div class="box s2 b-amount hidden"><p class="lab">${icon('coins')}Amount</p>${smudge()}<p class="ghostfig" aria-hidden="true"><span class="ghostnum">000</span><span class="unit">tTARI</span></p><span class="notcopy">${icon('lock-simple')}Not on this copy<span class="sr-only">: the amount is hidden from this holder</span></span></div>`;
    }
    const given = view === 'accountant' ? ', given to the accountant' : '';
    return `<div class="box s4 b-text"><p class="lab">${icon('file-text')}Invoice text${given}</p><p class="val">${esc(p.text)}</p></div>
      <div class="box s2 b-amount"><p class="lab">${icon('coins')}${AMOUNT_LABEL[view]}</p><p class="amount"><span class="big">${tari(p.amount_micro_tari)}</span><span class="unit">tTARI</span></p></div>`;
  }

  function partyBox(role, key, view) {
    const who = view === role
      ? `<p class="who me">${icon('user-circle')}This is your one-time key</p>`
      : `<p class="who">${icon('eye-slash')}Account behind it not on this copy</p>`;
    const name = role === 'client' ? 'Client' : 'Contractor';
    return `<div class="box s2 party"><p class="lab">${icon(role === 'client' ? 'buildings' : 'pen-nib')}${name}, one-time key</p><p class="val key mono-ish" title="${key}">${short(key, 8, 7)}</p>${who}</div>`;
  }

  function hashBox(i, view) {
    const live = state.book instanceof Map && state.book.get(i.no);
    const publicHash = live ? live.hash : i.invoice_hash;
    const check = view === 'accountant' ? `<dl class="check" id="acct-check">
        <dt>Salt</dt><dd>${esc(i.parties_only.salt)}</dd>
        <dt>sha256(salt|text)</dt><dd id="acct-hash">${sk('Computing the hash in your browser')}</dd>
        <dt>Result</dt><dd id="acct-result">${sk('Comparing with the book')}</dd>
        <dt>View key memo</dt><dd>${esc(i.parties_only.memo)}</dd>
      </dl>` : '';
    return `<div class="box s6"><p class="lab">${icon('hash')}Invoice hash${live ? ', read from the book' : ''}</p><p class="val mono-ish">${publicHash}</p>${check}</div>`;
  }

  async function runAccountantCheck(i) {
    const out = $('acct-hash');
    if (!out) return;
    const p = i.parties_only;
    const digest = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(`${p.salt}|${p.text}`));
    const hex = [...new Uint8Array(digest)].map((b) => b.toString(16).padStart(2, '0')).join('');
    const live = state.book instanceof Map && state.book.get(i.no);
    const target = live ? live.hash : i.invoice_hash;
    if (!$('acct-hash')) return;
    $('acct-hash').textContent = hex;
    $('acct-result').innerHTML = hex === target
      ? `<span class="match">${icon('seal-check')}Matches the ${live ? 'hash in the book, computed in your browser' : 'recorded invoice hash'}</span>`
      : `<span class="match no">${icon('warning')}Does not match the invoice hash</span>`;
  }

  function approvalLine(i, view) {
    if (i.approval_code) {
      return `<p class="code">${icon('key')}<span>Approval code, public in the book since approval: <b>${esc(i.approval_code)}</b>. On its own it cannot move the money: the claim also needs the contractor's key.</span></p>`;
    }
    if (view === 'client') return `<p class="code">${icon('key')}<span>You hold the approval code. It stays off chain until you approve.</span></p>`;
    return '';
  }

  function act(iconName, label, st, txId) {
    const done = Boolean(txId);
    return `<span class="act${done || st.startsWith('Possible') ? '' : ' off'}">${icon(iconName)}<span>${label}</span><span class="st">${st}</span>${done ? `<span class="tx">${txLink(txId, short(txId, 8, 0))}</span>` : ''}</span>`;
  }

  function clientActions(i, status) {
    const d = ep(i.deadline_epoch);
    const passed = epochKnown() && state.epoch >= i.deadline_epoch;
    const settledOrApproved = status === 'Approved' || status === 'Settled';
    const approve = tx(i, 'approve') ? 'Done'
      : status === 'Refunded' || passed ? 'No longer possible' : `Possible until epoch ${d}`;
    const refund = tx(i, 'refund') ? 'Done'
      : settledOrApproved ? 'No longer possible: approved'
      : passed ? 'Possible now' : `From epoch ${d}`;
    return act('lock-key', 'Fund', tx(i, 'fund') ? 'Done' : 'Next', tx(i, 'fund'))
      + act('check-circle', 'Approve', approve, tx(i, 'approve'))
      + act('arrow-counter-clockwise', 'Refund', refund, tx(i, 'refund'));
  }

  function contractorActions(i, status) {
    const claim = tx(i, 'claim') ? 'Done'
      : status === 'Refunded' ? 'No longer possible: refunded'
      : status === 'Approved' ? `Possible now, before epoch ${ep(i.deadline_epoch)}` : 'After approval';
    return act('hand-coins', 'Claim', claim, tx(i, 'claim'));
  }

  function accessBox(i, view, status) {
    const s = status || recordedStatus(i);
    if (view === 'public') {
      return `<h2>${icon('globe-simple')}This copy has no actions</h2><p class="lead">This is what anyone with the chain reads: hashes, one-time keys, the deadline and the status. No amount, no names, no invoice text.</p>${approvalLine(i, view)}`;
    }
    if (view === 'client') return `<h2>${icon('buildings')}Client actions</h2>${approvalLine(i, view)}<div class="actions">${clientActions(i, s)}</div>`;
    if (view === 'contractor') return `<h2>${icon('pen-nib')}Contractor actions</h2>${approvalLine(i, view)}<div class="actions">${contractorActions(i, s)}</div>`;
    return `<h2>${icon('calculator')}Accountant: reads, cannot spend</h2><p class="lead">The escrow has no key path, only its two conditional leaves. The view key reads the amount; it cannot move it.</p>`;
  }

  function sheetHTML(i, view) {
    const { status } = liveStatus(i);
    const word = STAMP[status || recordedStatus(i)];
    const yours = (r) => (view === r ? '<span class="yours">Your path</span>' : '');
    const d = ep(i.deadline_epoch);
    return `
      <div class="stamp" data-word="${word}">${stampSVG(word, word.length > 7 ? 196 : 178)}</div>
      <div class="grid">
        <div class="box s3 b-title"><p class="ttl">Hexcrow invoice</p><p class="small">Tari Ootle, Esmeralda testnet</p></div>
        <div class="box s1 b-no"><p class="lab">No.</p><p class="num">${pad4(i.no)}</p></div>
        ${statusBox(i)}
        <div class="box s3"><p class="lab">${icon('book-open')}Book</p><p class="val mono-ish" title="${state.run.book}">${short(state.run.book, 18, 6)}</p></div>
        <div class="box s3"><p class="lab">${icon('stack')}Template</p><p class="val mono-ish" title="${state.run.template}">${short(state.run.template, 17, 6)}</p></div>
        ${privateBoxes(i, view)}
        ${partyBox('client', i.client_key, view)}
        ${partyBox('contractor', i.contractor_key, view)}
        ${deadlineBox(i)}
        ${hashBox(i, view)}
        <div class="box s3"><p class="lab">${icon('cube')}Escrow commitment</p><p class="val mono-ish" title="${i.escrow_commitment}">${short(i.escrow_commitment, 12, 8)}</p></div>
        <div class="box s3"><p class="lab">${icon('tree-structure')}Condition root</p><p class="val mono-ish" title="${i.escrow_root}">${short(i.escrow_root, 12, 8)}</p></div>
        <div class="box s6 access">${accessBox(i, view, status)}</div>
        <div class="box s6 rules">
          <h2>${icon('scroll')}The rules</h2>
          <p class="fine">Public, enforced by the chain, not by Hexcrow.</p>
          <div class="rule-rows">
            <div class="rule-row">${icon('hand-coins')}<b>Claim ${yours('contractor')}</b><span>Approval code + contractor key + before epoch ${d}</span></div>
            <div class="rule-row">${icon('arrow-counter-clockwise')}<b>Refund ${yours('client')}</b><span>Client key + from epoch ${d}</span></div>
          </div>
        </div>
      </div>`;
  }

  function renderVersus(i) {
    const amount = `${tari(i.parties_only.amount_micro_tari)} tTARI`;
    const rows = [
      ['coins', 'Amount', `${amount}, in the contract balance`, 'Hidden in a commitment. The parties and the holders of the view key read it.'],
      ['buildings', 'Who paid', 'The client account, on the deposit', `A one-time key, ${short(i.client_key, 8, 0)}`],
      ['pen-nib', 'Who got paid', 'The contractor account, on the release', `A one-time key, ${short(i.contractor_key, 8, 0)}`],
      ['link', 'Other payments of the same people', 'Linkable through the whole history of both accounts', 'Linkable only if keys are reused. This demo reuses them for invoices 0001 and 0002.'],
      ['scroll', 'The rules and the status', 'Public', 'Public'],
    ];
    $('vs-body').innerHTML = rows.map(([ic, fact, open, ours]) =>
      `<tr><th scope="row">${icon(ic)}${fact}</th><td class="open">${esc(open)}</td><td>${esc(ours)}</td></tr>`).join('');
  }

  // ---- rendering: aside --------------------------------------------------------------------
  function renderInvoices() {
    $('invs').innerHTML = state.run.invoices.map((i) => {
      const { status } = liveStatus(i);
      const st = status ? `${status === 'Funded' ? `<span class="spin">${icon('hourglass-simple')}</span>` : icon(STATUS_ICON[status])}${status}` : sk('Reading the status');
      return `<li><button class="inv" type="button" data-no="${i.no}" aria-current="${i.no === state.invoice}">
        <span class="nm">Invoice ${pad4(i.no)}</span><span class="st">${st}</span>
        <span class="meta">Deadline epoch ${ep(i.deadline_epoch)}, ${i.timeline.length} transactions</span></button></li>`;
    }).join('');
  }

  function stubLabel(row, view, i) {
    if (row.kind === 'fund') {
      return SEES_PRIVATE[view] ? `Funded by the client, ${tari(i.parties_only.amount_micro_tari)} tTARI locked` : 'Funded by the client, amount hidden';
    }
    return row.kind === 'attack' ? `Attack: ${row.label[0].toLowerCase()}${row.label.slice(1)}` : row.label;
  }

  function renderStubs() {
    const i = inv();
    const rows = i.timeline;
    const rejected = rows.filter((r) => !r.accepted).length;
    $('tl-h').textContent = `Invoice ${pad4(i.no)} on chain`;
    $('tl-count').innerHTML = `${rows.length} transactions on Esmeralda: ${rows.length - rejected} committed, ${rejected} rejected. Fees in µT, 1 tTARI = 1,000,000 µT. <button class="recheck" id="recheck" type="button">${icon('arrows-clockwise')}Check again on chain</button>`;
    // The entrance plays once per invoice; live refreshes repaint without replaying it.
    $('stubs').classList.toggle('settled', state.paintedInvoice === i.no);
    state.paintedInvoice = i.no;
    $('stubs').innerHTML = rows.map((r, n) => {
      const v = state.verdicts.get(r.tx) || { state: 'wait', text: 'Checking on chain' };
      const verdictIcon = { ok: 'seal-check', bad: 'warning', off: 'cloud-slash', wait: 'circle-notch' }[v.state];
      const fee = r.fee_micro_tari ? `<span>Fee ${num(r.fee_micro_tari)} µT</span>` : '<span>No fee</span>';
      return `<li class="stub ${r.accepted ? 'ok' : 'rej'}" style="--i:${Math.min(n, 7)}">
        <span class="i">${n + 1}</span>
        <p class="what">${esc(stubLabel(r, state.view, i))}${r.kind === 'attack' ? ` <span class="who">signed by the ${esc(r.actor)}</span>` : ''}</p>
        ${r.why ? `<p class="why">${esc(r.why)}</p>` : ''}
        <p class="meta">${txLink(r.tx)}${fee}</p>
        <p class="meta"><span class="ver" data-state="${v.state}">${icon(verdictIcon)}${esc(v.text)}</span></p>
        <span class="mini">${stampSVG(r.accepted ? 'COMMITTED' : 'REJECTED', r.accepted ? 196 : 176)}</span>
      </li>`;
    }).join('');
    $('recheck').addEventListener('click', () => verifyAll(true));
  }

  // ---- render + transitions ----------------------------------------------------------------
  function render() {
    const i = inv();
    document.documentElement.dataset.view = state.view;
    const sheet = $('sheet');
    sheet.innerHTML = sheetHTML(i, state.view);
    sheet.setAttribute('aria-labelledby', 'tab-' + state.view);
    renderInvoices();
    renderStubs();
    renderVersus(i);
    runAccountantCheck(i);
  }

  function writeURL() {
    const q = new URLSearchParams({ invoice: String(state.invoice), view: state.view });
    history.replaceState(null, '', `?${q}`);
  }

  function slideSheet(prevCopy) {
    if (reduce.matches) return;
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

  function setView(view, silent) {
    if (view === state.view) return;
    const after = slideSheet(COPY[state.view]);
    state.view = view;
    document.querySelectorAll('.tab').forEach((t) => {
      const on = t.dataset.v === view;
      t.setAttribute('aria-selected', String(on));
      t.tabIndex = on ? 0 : -1;
    });
    render();
    if (after) after();
    writeURL();
    if (!silent) sound.play('switch');
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
  }

  function readURL() {
    const q = new URLSearchParams(location.search);
    const v = q.get('view');
    const n = Number(q.get('invoice'));
    if (VIEWS.includes(v)) state.view = v;
    if (state.run.invoices.some((i) => i.no === n)) state.invoice = n;
    document.querySelectorAll('.tab').forEach((t) => {
      const on = t.dataset.v === state.view;
      t.setAttribute('aria-selected', String(on));
      t.tabIndex = on ? 0 : -1;
    });
  }

  // On first load, show each copy once so the four views are seen without a click.
  // Any input from the visitor stops it for good.
  function tour() {
    if (reduce.matches || new URLSearchParams(location.search).has('view')) return;
    const order = ['client', 'contractor', 'accountant', 'public'];
    const inputs = ['pointerdown', 'keydown', 'wheel', 'touchstart'];
    let step = 0;
    let timer = null;
    const stop = () => {
      clearTimeout(timer);
      inputs.forEach((ev) => removeEventListener(ev, stop, true));
    };
    inputs.forEach((ev) => addEventListener(ev, stop, true));
    const next = () => {
      setView(order[step], true);
      step += 1;
      if (step < order.length) timer = setTimeout(next, TOUR_STEP_MS);
      else stop();
    };
    timer = setTimeout(next, TOUR_START_MS);
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
    $('f-tests').textContent = `${state.run.engine_tests_passed} engine tests pass (local run of the compiled template)`;
    readURL();
    bind();
    render();
    refreshNetwork();
    verifyAll(false);
    setInterval(refreshNetwork, REFRESH_MS);
    tour();
  }

  start();
})();
