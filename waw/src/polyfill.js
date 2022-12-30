// This file is loaded on both the main thread and in the worklet.

// `TextDecoder` & `TextEncoder` Polyfill
if (!globalThis.TextDecoder) {
  globalThis.TextDecoder = class TextDecoder {
    // https://gist.github.com/Yaffle/5458286
    decode(octets) {
      if (typeof octets == "undefined") {
        return "";
      }
      var string = "";
      var i = 0;
      while (i < octets.length) {
        var octet = octets[i];
        var bytesNeeded = 0;
        var codePoint = 0;
        if (octet <= 0x7f) {
          bytesNeeded = 0;
          codePoint = octet & 0xff;
        } else if (octet <= 0xdf) {
          bytesNeeded = 1;
          codePoint = octet & 0x1f;
        } else if (octet <= 0xef) {
          bytesNeeded = 2;
          codePoint = octet & 0x0f;
        } else if (octet <= 0xf4) {
          bytesNeeded = 3;
          codePoint = octet & 0x07;
        }
        if (octets.length - i - bytesNeeded > 0) {
          var k = 0;
          while (k < bytesNeeded) {
            octet = octets[i + k + 1];
            codePoint = (codePoint << 6) | (octet & 0x3f);
            k += 1;
          }
        } else {
          codePoint = 0xfffd;
          bytesNeeded = octets.length - i;
        }
        string += String.fromCodePoint(codePoint);
        i += bytesNeeded + 1;
      }
      return string;
    }
  };
}

if (!globalThis.TextEncoder) {
  globalThis.TextEncoder = class TextEncoder {
    // https://gist.github.com/Yaffle/5458286
    encode(string) {
      if (typeof string == "undefined") {
        return new Uint8Array(0);
      }
      var octets = [];
      var length = string.length;
      var i = 0;
      while (i < length) {
        var codePoint = string.codePointAt(i);
        var c = 0;
        var bits = 0;
        if (codePoint <= 0x0000007f) {
          c = 0;
          bits = 0x00;
        } else if (codePoint <= 0x000007ff) {
          c = 6;
          bits = 0xc0;
        } else if (codePoint <= 0x0000ffff) {
          c = 12;
          bits = 0xe0;
        } else if (codePoint <= 0x001fffff) {
          c = 18;
          bits = 0xf0;
        }
        octets.push(bits | (codePoint >> c));
        c -= 6;
        while (c >= 0) {
          octets.push(0x80 | ((codePoint >> c) & 0x3f));
          c -= 6;
        }
        i += codePoint >= 0x10000 ? 2 : 1;
      }
      return octets;
    }
  };
}

// `Worklet.prototype.addModule` Polyfill (Firefox)
// https://gist.github.com/lukaslihotzki/b50ccb61ff3a44b48fc4d5ed7e54303f

if (globalThis.Worklet) {
  const wrappedFunc = Worklet.prototype.addModule;

  Worklet.prototype.addModule = async function (url) {
    try {
      return await wrappedFunc.call(this, url);
    } catch (e) {
      if (e.name != "AbortError") {
        throw e;
      }
      // Assume error is caused by https://bugzilla.mozilla.org/show_bug.cgi?id=1572644
      console.warn(
        "Worklet.addModule call failed, transpiling... https://bugzilla.mozilla.org/show_bug.cgi?id=1572644"
      );

      const esbuild = await import(
        "https://unpkg.com/esbuild-wasm@0.11.12/esm/browser.min.js"
      );

      if (!globalThis.__esbuild_init) {
        await esbuild.initialize({
          wasmURL: "https://unpkg.com/esbuild-wasm@0.11.12/esbuild.wasm",
        });

        globalThis.__esbuild_init = true;
      }

      const result = await esbuild.build({
        entryPoints: [url],
        plugins: [
          {
            name: "resolve",
            setup: (build) => {
              build.onResolve({ filter: /.*/ }, async (args) => {
                return {
                  namespace: "a",
                  path: new URL(
                    args.path,
                    new URL(window.location.href)
                  ).toString(),
                };
              });

              build.onLoad({ filter: /.*/ }, async (args) => {
                const contents = await fetch(args.path).then((response) =>
                  response.text()
                );
                return {
                  contents,
                };
              });
            },
          },
        ],
        bundle: true,
        write: false,
      });

      const blob = new Blob([result.outputFiles[0].text], {
        type: "text/javascript",
      });
      const objectUrl = URL.createObjectURL(blob);
      try {
        return await wrappedFunc.call(this, objectUrl);
      } finally {
        URL.revokeObjectURL(objectUrl);
      }
    }
  };
}

export function nop() {}
