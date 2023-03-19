const {
  performance,
  PerformanceObserver,
  createHistogram,
} = require("node:perf_hooks");
const lookup = require("dns/promises").lookup;
const ffiLookup = require(".").lookup;

const ffiHist = createHistogram();
const nativeHist = createHistogram();

const HOST = "google.com.";
const ALL = false;

main();

async function main() {
  const obs = new PerformanceObserver((list, observer) => {
    list.getEntries().forEach((item) => {
      const duration = Math.round(item.duration);
      if (duration > 0) {
        if (item.name.startsWith("ffi")) {
          ffiHist.record(duration);
        } else {
          nativeHist.record(duration);
        }
      }
    });
  });

  obs.observe({ type: "measure" });

  for (let i = 0; i < 1000; i++) {
    await testNative(i);
    await testFFI(i);
  }

  obs.disconnect();

  function msToSec(n) {
    return n / 1000;
  }

  console.log(`FFI max ${msToSec(ffiHist.max)}`);
  console.log(`Native max ${msToSec(nativeHist.max)}`);

  console.log();

  console.log(`FFI min ${msToSec(ffiHist.min)}`);
  console.log(`Native min ${msToSec(nativeHist.min)}`);

  console.log();

  console.log(`FFI p99 ${msToSec(ffiHist.percentile(99))}`);
  console.log(`Native p99 ${msToSec(nativeHist.percentile(99))}`);

  console.log();

  console.log(`FFI p90 ${msToSec(ffiHist.percentile(90))}`);
  console.log(`Native p90 ${msToSec(nativeHist.percentile(90))}`);

  console.log();

  console.log(`FFI p50 ${msToSec(ffiHist.percentile(50))}`);
  console.log(`Native p50 ${msToSec(nativeHist.percentile(50))}`);
}

function testNative(n) {
  performance.mark(`native-${n}-start`);
  return lookup(HOST, { all: ALL })
    .then((data) => {
      console.log(`${n} Native: ${JSON.stringify(data)}`);
      performance.mark(`native-${n}-end`);
      performance.measure(
        "native start to end",
        `native-${n}-start`,
        `native-${n}-end`
      );
    })
    .catch((err) => console.error(err));
}

function testFFI(n) {
  performance.mark(`ffi-${n}-start`);
  return ffiLookup(HOST, 4, ALL)
    .then((data) => {
      console.log(`${n} FFI: ${JSON.stringify(data)}`);
      performance.mark(`ffi-${n}-end`);
      performance.measure("ffi start to end", `ffi-${n}-start`, `ffi-${n}-end`);
    })
    .catch((err) => console.error(err));
}
