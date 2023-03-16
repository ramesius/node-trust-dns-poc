const { lookup } = require("dns");
const https = require("https");
const ffiLookup = require(".").lookup;

https.globalAgent = new https.Agent({ lookup: lookupOverride.bind(https.globalAgent) });

const USE_FFI = true;
const NS_PER_SEC = 1e9;
const MS_PER_NS = 1e6;

function lookupOverride(hostname, options, callback) {
  console.log("lookupOverride", options, hostname);

  const { all, family, hints, verbatim } = options;

  if (USE_FFI) {
    ffiLookup(hostname, { all, family })
      .then((addressResult) => {
        console.log("FFI lookup result", addressResult);
        callback(null, addressResult.address, addressResult.family);
      })
      .catch((err) => callback(err, null));
  } else {
    lookup(hostname, { all, family, hints, family }, (err, results, family) => {
      console.log("Native lookup result", results);
      callback(err, results, family);
    });
  }
}

const eventTimes = {
  // use process.hrtime() as it's not a subject of clock drift
  startAt: process.hrtime(),
  dnsLookupAt: undefined,
  tcpConnectionAt: undefined,
  tlsHandshakeAt: undefined,
  firstByteAt: undefined,
  endAt: undefined,
};

function getTimings(eventTimes) {
  return {
    // There is no DNS lookup with IP address
    dnsLookup:
      eventTimes.dnsLookupAt !== undefined
        ? getHrTimeDurationInMs(eventTimes.startAt, eventTimes.dnsLookupAt)
        : undefined,
    tcpConnection: getHrTimeDurationInMs(
      eventTimes.dnsLookupAt || eventTimes.startAt,
      eventTimes.tcpConnectionAt
    ),
    // There is no TLS handshake without https
    tlsHandshake:
      eventTimes.tlsHandshakeAt !== undefined
        ? getHrTimeDurationInMs(
            eventTimes.tcpConnectionAt,
            eventTimes.tlsHandshakeAt
          )
        : undefined,
    // firstByte: getHrTimeDurationInMs(
    //   eventTimes.tlsHandshakeAt || eventTimes.tcpConnectionAt,
    //   eventTimes.firstByteAt
    // ),
    // contentTransfer: getHrTimeDurationInMs(
    //   eventTimes.firstByteAt,
    //   eventTimes.endAt
    // ),
    total: getHrTimeDurationInMs(eventTimes.startAt, eventTimes.endAt),
  };
}

/**
 * Get duration in milliseconds from process.hrtime()
 * @function getHrTimeDurationInMs
 * @param {Array} startTime - [seconds, nanoseconds]
 * @param {Array} endTime - [seconds, nanoseconds]
 * @return {Number} durationInMs
 */
function getHrTimeDurationInMs(startTime, endTime) {
  const secondDiff = endTime[0] - startTime[0];
  const nanoSecondDiff = endTime[1] - startTime[1];
  const diffInNanoSecond = secondDiff * NS_PER_SEC + nanoSecondDiff;

  return diffInNanoSecond / MS_PER_NS;
}

const req = https.request(
  {
    host: "www.google.com",
    path: "/",
    method: "GET",
    protocol: "https:",
  },
  (response) => {
    console.log("response.statusCode", response.statusCode);

    response.on("data", (d) => {});
    response.on("end", () => {
      eventTimes.endAt = process.hrtime();

      console.log(getTimings(eventTimes));
    });
  }
);

req.on("socket", (socket) => {
  socket.on("lookup", (err, address, family, host) => {
    console.log("on socket lookup", err, address, family, host, this);
    eventTimes.dnsLookupAt = process.hrtime();
  });
  socket.on("connect", () => {
    eventTimes.tcpConnectionAt = process.hrtime();
  });
  socket.on("secureConnect", () => {
    eventTimes.tlsHandshakeAt = process.hrtime();
  });
  socket.on("timeout", () => {
    req.abort();

    const err = new Error("ETIMEDOUT");
    err.code = "ETIMEDOUT";
    callback(err);
  });
});

req.on("error", (e) => {
  console.error(e);
});

req.end();
