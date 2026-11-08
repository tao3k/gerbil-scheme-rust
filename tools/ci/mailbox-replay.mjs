import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { closeSync, mkdirSync, openSync, readFileSync, realpathSync, writeFileSync } from 'node:fs';
import { release } from 'node:os';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const rate = 1_000_000;
const warmupRate = 1_000;
const requestDeadlineSeconds = 10;
const latencyBucketNs = 1_000;
const readerPin = 'f1e24bfcdb8a43cdd85a1fb1b366655c8047b041756e0a53f20325a1cc732d86';
const controlPin = '53ba9b24871cb1abd5b7e705bc04be83f1a1630bc7c22c7997db930aaed98e8d';
// Complete Actor C, object and VM lifecycle are bound by the paired Git provenance receipt.
const verifiedMatchedPin = '595f19cbef1cd0d4ad1cd3501e6038b30911476bf4027c8eb42c4fd728473b5d';
const verifiedCandidatePin = '2f0b96f960b2d4db9a01d461a14edc9165e7519c1bfa5be24cc39748d22832ff';
const controlEnvelope = { revision: '3477a02', p99: 15_000_000, p999: 15_000_000,
  peak: 12_969, throughput: 987_434.2416438768 };
const published = { revision: "d56f94c", p99: 4_513_000, p999: 4_819_000,
  peak: 4_853, throughput: 999_944.8780385981 };
const absolute = { p99: 418_000, peak: 128, whole: 3_000_000_000 };
const requiredControls = 3;
const startupEnvironmentKeys = [
  'DYLD_FALLBACK_FRAMEWORK_PATH', 'DYLD_FALLBACK_LIBRARY_PATH', 'DYLD_FRAMEWORK_PATH',
  'DYLD_INSERT_LIBRARIES', 'DYLD_LIBRARY_PATH', 'DYLD_PRINT_LIBRARIES', 'DYLD_ROOT_PATH',
  'GAMBOPT', 'GERBIL_HOME', 'GERBIL_PATH', 'TMPDIR',
];
const childTimeoutMs = 30_000;
const args = ['--capacity-research', '--backpressure-owned-schedule',
  '--rate=1000000', '--seconds=1', '--require-native-acceptance'];
const expectedPanic = 'native 500k performance admission failed; cleanup success is independent';

function loadEvents(stdout) {
  return stdout.split('\n').filter(line => line.startsWith('{')).map(line => JSON.parse(line));
}

function complete(row, count, warmup) {
  return row?.warmup === warmup && row.rate === count && row.seconds === 1
    && row.planned === count && row.accepted === count && row.completed === count
    && row.rejected === 0 && row.failed === 0 && row.accepted_timeouts === 0
    && row.remaining_native === 0 && row.remaining_staged === 0
    && row.watchdog_seconds === requestDeadlineSeconds
    && row.latency_bucket_upper_bound_ns === latencyBucketNs
    && row.owned_source_schedule === true && row.owners === 1 && row.native_active === 3
    && row.scheme_workers === 4 && row.window === 128
    && row.diagnostic_native_observation === false && row.diagnostic_owner_watch === false
    && row.diagnostic_no_actor_handoff === false && row.scoped_user_initiated_qos === false
    && row.native_source_park === false && row.native_coalesced_wake === false
    && row.ready_drain_pipeline === false && row.rust_per_message_entries === 0
    && row.defaults_changed === false && row.data_access_edge_contract === true;
}

function consistentTiming(row) {
  const { elapsed_ns: elapsed, offered_elapsed_ns: offered, whole_native_call_ns: whole,
    completed_per_second: throughput, source_backpressure: source } = row;
  const lastPublication = source?.last_publication_elapsed_ns;
  const drain = source?.drain_after_schedule_ns;
  if (![elapsed, offered, whole, lastPublication, drain].every(Number.isSafeInteger)
      || offered !== (row.planned - 1) * (1_000_000_000 / row.rate)
      || lastPublication < offered || elapsed < lastPublication || whole < elapsed
      || drain !== elapsed - offered || !Number.isFinite(throughput) || throughput <= 0) {
    return false;
  }

  // The pinned Rust reader divides by Duration::as_secs_f64().
  const computed = row.completed / (elapsed / 1_000_000_000);
  return Math.abs(throughput - computed) <= 4 * Number.EPSILON * computed;
}

export function judge({ stdout, stderr, status, signal, error }) {
  const events = loadEvents(stdout);
  const [warmup, main, extra] = events.filter(row => row.event === 'gerbil_actor_arrival');
  const cleanups = events.filter(row => row.event === 'gerbil_actor_cleanup');
  const terminalFailures = events.filter(row => row.event === 'gerbil_actor_main_failure'
    && row.warmup === false);
  const failure = { relativeQualified: false, absolutePassed: false, main };
  if (terminalFailures.length === 1) failure.terminalFailure = terminalFailures[0];
  if (error || signal || extra || !complete(warmup, warmupRate, true)
      || !complete(main, rate, false) || !consistentTiming(warmup) || !consistentTiming(main)
      || cleanups.length !== 1 || cleanups[0].clean !== true
      || events.some(row => row.diagnostic_only === true
        || row.event === 'native_clock_anchor' || row.event?.includes('failure'))
      || cleanups[0] !== events.at(-1)) return failure;

  const p99 = main.successful_planned_to_consumed_p99_ns;
  const p999 = main.successful_planned_to_consumed_p999_ns;
  const peak = main.source_backpressure?.maximum_due_unconsumed_messages;
  const throughput = main.completed_per_second;
  const whole = main.whole_native_call_ns;
  if (![p99, p999, peak].every(Number.isSafeInteger)
      || p99 <= 0 || p999 < p99 || peak < 0
      || p99 % latencyBucketNs !== 0 || p999 % latencyBucketNs !== 0
      || main.source_backpressure.original_window_guard_passed !== (peak <= absolute.peak)) return failure;

  const absolutePassed = p99 <= absolute.p99 && peak <= absolute.peak && whole <= absolute.whole;
  if (main.performance_admission?.passed !== absolutePassed) return failure;
  const cleanExit = absolutePassed ? status === 0 : status === 101 && stderr.includes(expectedPanic);
  if (!cleanExit) return failure;
  const relativeFailures = [];
  if (whole > absolute.whole) relativeFailures.push('whole-native-call');
  if (p99 > controlEnvelope.p99) relativeFailures.push('p99');
  if (p999 > controlEnvelope.p999) relativeFailures.push('p99.9');
  if (peak > controlEnvelope.peak) relativeFailures.push('sampled-peak');
  if (throughput < controlEnvelope.throughput) relativeFailures.push('successful-throughput');
  // A complete raw op62 turn can retire at most one request between samples.
  // A terminal or instrumented receipt cannot use that completion bound.
  const backlog = { sampledPeak: peak, truePeakUpper: peak + 1 };
  return { main, cleanShutdown: true, backlog, absolutePassed,
    relativeQualified: relativeFailures.length === 0, relativeFailures };
}

function measured(row) {
  return {
    p99: row.successful_planned_to_consumed_p99_ns,
    p999: row.successful_planned_to_consumed_p999_ns,
    peak: row.source_backpressure.maximum_due_unconsumed_messages,
    throughput: row.completed_per_second,
  };
}

function improves(control, candidate) {
  return candidate.p99 < control.p99 && candidate.peak + 1 < control.peak
    && candidate.p999 <= control.p999 && candidate.throughput >= control.throughput;
}

export function compare(run) {
  for (let index = 1; index <= requiredControls; index++) {
    if (!run(`frozen-${index}`, 'frozen').relativeQualified) return 'stopped-at-frozen-control';
  }
  const matched = run('matched-control', 'matched');
  if (!matched.relativeQualified) return 'stopped-at-matched-control';
  const candidate = run('candidate', 'candidate');
  if (!candidate.absolutePassed || !candidate.relativeQualified) return 'candidate-failed';
  const result = measured(candidate.main);
  // One retirement between samples can hide one outstanding request.
  if (result.peak + 1 > absolute.peak) return 'candidate-true-peak-unverified';
  if (!improves(published, result)) return 'candidate-no-published-gain';
  return improves(measured(matched.main), result) ? 'passed' : 'candidate-no-matched-gain';
}

function sha256(value) {
  return createHash('sha256').update(value).digest('hex');
}

function hash(file) {
  return sha256(readFileSync(file));
}

function launchProtocol(environment) {
  // Fingerprint a narrow allowlist without serializing machine-local paths.
  // This records the parent-supplied configuration, not dyld's decisions.
  const startupEnvironment = Object.fromEntries(startupEnvironmentKeys.map(key =>
    [key, environment[key] === undefined ? null : sha256(environment[key])]));
  return {
    node: process.version, platform: process.platform, arch: process.arch, kernel: release(),
    cwdSha256: sha256(realpathSync('.')), startupEnvironment,
    lifecycleTrace: environment.GERBIL_INSTANCE_LIFECYCLE_TRACE !== undefined,
    stdin: 'ignore', stdout: 'file', stderr: 'file', timeoutMs: childTimeoutMs,
  };
}

function check(pin, file) {
  if (hash(file) !== pin) throw new Error(`SHA-256 mismatch for ${file}`);
}

function main(argv) {
  if (argv.length !== 7) throw new Error(
    'usage: mailbox-replay.mjs READER FROZEN_IMAGE MATCHED_IMAGE MATCHED_SHA256 CANDIDATE_IMAGE CANDIDATE_SHA256 RECEIPT_DIR');
  const [readerPath, controlPath, matchedPath, matchedPin, candidatePath, candidatePin, receiptDir] = argv;
  if (matchedPin !== verifiedMatchedPin || candidatePin !== verifiedCandidatePin) {
    throw new Error('image pins must match the verified d56f94c/df1491e op62 pair');
  }
  const reader = realpathSync(readerPath);
  const images = {
    frozen: { image: realpathSync(controlPath), pin: controlPin },
    matched: { image: realpathSync(matchedPath), pin: matchedPin },
    candidate: { image: realpathSync(candidatePath), pin: candidatePin },
  };
  check(readerPin, reader);
  for (const { image, pin } of Object.values(images)) check(pin, image);
  const out = resolve(receiptDir);
  mkdirSync(out);
  const environment = { ...process.env };
  const plan = { args, published, controlEnvelope, absolute, requiredControls, readerPin, controlPin, matchedPin, candidatePin,
    launchProtocol: launchProtocol(environment) };
  writeFileSync(resolve(out, 'plan.json'), JSON.stringify(plan, null, 2) + '\n', { flag: 'wx' });
  const runs = [];
  const decision = compare((label, role) => {
    const { image, pin } = images[role];
    check(readerPin, reader);
    check(pin, image);
    const stdoutPath = resolve(out, label + '.stdout');
    const stderrPath = resolve(out, label + '.stderr');
    const stdoutFd = openSync(stdoutPath, 'wx');
    const stderrFd = openSync(stderrPath, 'wx');
    let result;
    let childElapsedNs;
    const started = process.hrtime.bigint();
    try {
      result = spawnSync(reader, args, {
        env: { ...environment, GERBIL_SERVICE_BENCH_IMAGE: image },
        stdio: ['ignore', stdoutFd, stderrFd], timeout: childTimeoutMs,
      });
      childElapsedNs = Number(process.hrtime.bigint() - started);
    } finally {
      closeSync(stdoutFd);
      closeSync(stderrFd);
    }
    check(readerPin, reader);
    check(pin, image);
    const { status, signal, error } = result;
    const stdout = readFileSync(stdoutPath, 'utf8');
    const stderr = readFileSync(stderrPath, 'utf8');
    let judgment;
    try {
      judgment = judge({ stdout, stderr, status, signal, error });
    } catch (invalid) {
      judgment = { relativeQualified: false, absolutePassed: false, receiptError: invalid.message };
    }
    runs.push({ label, imageSha256: pin, stdoutSha256: hash(stdoutPath), stderrSha256: hash(stderrPath),
      status, signal, error: error?.message, childElapsedNs, ...judgment });
    writeFileSync(resolve(out, 'runs.json'), JSON.stringify(runs, null, 2) + '\n');
    return judgment;
  });
  writeFileSync(resolve(out, 'decision.json'), JSON.stringify({ decision }, null, 2) + '\n', { flag: 'wx' });
  console.log(decision);
  if (decision !== 'passed') process.exitCode = 1;
}

if (process.argv[1] && pathToFileURL(realpathSync(process.argv[1])).href === import.meta.url) {
  main(process.argv.slice(2));
}
