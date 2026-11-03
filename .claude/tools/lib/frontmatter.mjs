// core/tools/lib/frontmatter.mjs — a dependency-free YAML-subset parser for
// the frontmatter my-aidlc authors. It supports the shapes used by phases,
// stages, scopes, and agents: scalars, booleans, numbers, inline lists,
// block lists, and lists of maps. It is intentionally small and strict; it is
// not a general YAML implementation.

/** Parse `---\n...\n---` frontmatter plus the remaining body. */
export function parseFrontmatter(text) {
  const match = /^---\r?\n([\s\S]*?)\r?\n---\r?\n?/.exec(text);
  if (!match) return { data: {}, body: text, hasFrontmatter: false };
  return {
    data: parseYaml(match[1]),
    body: text.slice(match[0].length),
    hasFrontmatter: true,
  };
}

/** Parse a YAML subset into a plain object. */
export function parseYaml(source) {
  const rawLines = source.split(/\r?\n/);
  const lines = [];
  for (const raw of rawLines) {
    const noComment = raw.replace(/\s+#.*$/, "");
    if (noComment.trim() === "" || noComment.trimStart().startsWith("#")) continue;
    lines.push(noComment.replace(/\s+$/, ""));
  }

  let index = 0;

  const indentOf = (line) => line.length - line.trimStart().length;
  const matchKeyValue = (s) => {
    const m = /^([A-Za-z0-9_-]+):(?:[ \t]+(.*))?$/.exec(s);
    if (!m) return null;
    return { key: m[1], value: m[2] === undefined ? "" : m[2].trim() };
  };

  function parseScalar(value) {
    if (value === "" || value === "~" || value === "null") return value === "" ? "" : null;
    if (value === "true") return true;
    if (value === "false") return false;
    if (/^-?\d+$/.test(value)) return Number.parseInt(value, 10);
    if (/^".*"$/.test(value) || /^'.*'$/.test(value)) return value.slice(1, -1);
    if (value === "[]") return [];
    if (value.startsWith("[") && value.endsWith("]")) {
      return value
        .slice(1, -1)
        .split(",")
        .map((part) => parseScalar(part.trim()))
        .filter((part) => part !== "");
    }
    return value;
  }

  function parseBlockScalar(indent, style) {
    const parts = [];
    while (index < lines.length) {
      const line = lines[index];
      if (indentOf(line) < indent) break;
      index += 1;
      parts.push(line.slice(indent));
    }
    if (style === ">") {
      return parts.join(" ").replace(/\s+/g, " ").trim();
    }
    return parts.join("\n");
  }

  function parseList(indent) {
    const out = [];
    while (index < lines.length) {
      const line = lines[index];
      if (indentOf(line) < indent) break;
      const trimmed = line.trimStart();
      if (trimmed === "-" || trimmed.startsWith("- ")) {
        const rest = trimmed.slice(1).trim();
        const itemIndent = indentOf(line);
        if (rest === "") {
          index += 1;
          out.push(parseBlock(indent + 2));
          continue;
        }
        const kv = matchKeyValue(rest);
        if (kv) {
          const obj = {};
          obj[kv.key] = kv.value === "" ? null : parseScalar(kv.value);
          index += 1;
          while (index < lines.length) {
            const cont = lines[index];
            const contIndent = indentOf(cont);
            if (contIndent <= itemIndent) break;
            const contKv = matchKeyValue(cont.trimStart());
            index += 1;
            if (contKv) {
              if (contKv.value === "") {
                const next = lines[index];
                obj[contKv.key] =
                  next !== undefined && indentOf(next) > contIndent
                    ? parseBlock(indentOf(next))
                    : null;
              } else {
                obj[contKv.key] = parseScalar(contKv.value);
              }
            }
          }
          out.push(obj);
        } else {
          index += 1;
          out.push(parseScalar(rest));
        }
      } else {
        break;
      }
    }
    return out;
  }

  function parseMap(indent) {
    const out = {};
    while (index < lines.length) {
      const line = lines[index];
      const lineIndent = indentOf(line);
      if (lineIndent < indent) break;
      if (lineIndent > indent) {
        index += 1;
        continue;
      }
      const kv = matchKeyValue(line.trimStart());
      if (!kv) {
        index += 1;
        continue;
      }
      index += 1;
      if (kv.value === ">" || kv.value === "|") {
        out[kv.key] = parseBlockScalar(indent + 2, kv.value);
      } else if (kv.value === "") {
        const next = lines[index];
        if (next !== undefined && indentOf(next) > indent) {
          out[kv.key] = parseBlock(indentOf(next));
        } else {
          out[kv.key] = null;
        }
      } else {
        out[kv.key] = parseScalar(kv.value);
      }
    }
    return out;
  }

  function parseBlock(indent) {
    const next = lines[index];
    if (next === undefined) return null;
    const trimmed = next.trimStart();
    if (trimmed === "-" || trimmed.startsWith("- ")) return parseList(indent);
    return parseMap(indent);
  }

  const result = parseBlock(0);
  return result === null ? {} : result;
}
