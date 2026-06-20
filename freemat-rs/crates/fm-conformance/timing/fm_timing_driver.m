% fm_timing_driver — time every test_*.m in a directory on the ORIGINAL FreeMat
% interpreter, mirroring the Rust harness's methodology so the two are directly
% comparable. For each test:
%   * run it once to classify the outcome (pass / fail / error), and
%   * if it did not error, repeat the call under a fixed wall-clock BUDGET using
%     adaptive batching (so timer overhead is amortised and fast tests get many
%     iterations, slow tests few), reporting mean nanoseconds-per-invocation.
%
% Only the repeated call is timed — interpreter startup and path loading are
% excluded, matching `time_test_file` on the Rust side.
%
% Args:
%   dirpath  full path to the test directory
%   dirname  short directory label written into the CSV (e.g. 'flow')
%   budget_s per-test timing budget in seconds (e.g. 0.1)
%   max_reps hard cap on iterations per test
%   outfile  CSV file to append rows to (header written by the orchestrator)
function fm_timing_driver(dirpath, dirname, budget_s, max_reps, outfile)
  d = dir([dirpath '/test_*.m']);
  fid = fopen(outfile, 'w');
  fprintf(fid, 'dir,name,outcome,reps,ns_per_iter\n');
  for i = 1:length(d)
    nm = d(i).name;
    stem = nm(1:end-2);            % strip the trailing '.m'

    outcome = classify(stem);
    reps = 0;
    total = 0.0;
    if ~strcmp(outcome, 'error')
      % Estimate per-call time by growing a warmup batch until it clears the
      % timer's resolution floor (a single fast call reads as 0). Size the real
      % batch for ~10 ms so per-batch tic/toc overhead stays under 1% while
      % bounding overshoot on slow tests.
      Bw = 1;
      tic; runq(stem); e = toc;
      while e < 0.005 && Bw < 16384
        Bw = Bw * 4;
        tic;
        for k = 1:Bw
          runq(stem);
        end
        e = toc;
      end
      t0 = e / Bw;
      if t0 <= 0; t0 = 1e-7; end
      B = max(1, round(0.01 / t0));
      B = min(B, 100000);
      while total < budget_s && reps < max_reps
        tic;
        for k = 1:B
          runq(stem);
        end
        total = total + toc;
        reps = reps + B;
      end
    end

    if reps > 0
      nspi = total / reps * 1e9;
    else
      nspi = 0.0;
    end
    fprintf(fid, '%s,%s,%s,%d,%.1f\n', dirname, stem, outcome, reps, nspi);
  end
  fclose(fid);

% Call the test once, swallowing any error (used inside the timing loop).
function runq(stem)
  try
    feval(stem);
  catch
  end

% Classify the first run exactly as the Rust harness does: an error/throw is
% 'error'; an empty or all-zero result is 'fail'; an all-nonzero result is
% 'pass'.
function o = classify(stem)
  o = 'error';
  try
    r = feval(stem);
    if isempty(r)
      o = 'fail';
    elseif all(r(:) ~= 0)
      o = 'pass';
    else
      o = 'fail';
    end
  catch
    o = 'error';
  end
