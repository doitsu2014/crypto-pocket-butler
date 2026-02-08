import fs from 'node:fs/promises';
import path from 'node:path';

const NOTION_KEY = process.env.NOTION_KEY;
const NOTION_VER = '2025-09-03';

if (!NOTION_KEY) {
  console.error('Missing NOTION_KEY');
  process.exit(2);
}

const api = (p) => `https://api.notion.com/v1${p}`;

async function http(method, p, body) {
  const res = await fetch(api(p), {
    method,
    headers: {
      'Authorization': `Bearer ${NOTION_KEY}`,
      'Notion-Version': NOTION_VER,
      'Content-Type': 'application/json'
    },
    body: body ? JSON.stringify(body) : undefined,
  });
  if (!res.ok) {
    const t = await res.text();
    throw new Error(`${method} ${p} -> ${res.status} ${res.statusText}: ${t.slice(0,500)}`);
  }
  return res.json();
}

async function getAllChildren(blockId, pageSize = 100) {
  let out = [];
  let cursor = undefined;
  while (true) {
    const qs = new URLSearchParams({ page_size: String(pageSize) });
    if (cursor) qs.set('start_cursor', cursor);
    const res = await http('GET', `/blocks/${blockId}/children?${qs}`);
    out = out.concat(res.results ?? []);
    if (!res.has_more) break;
    cursor = res.next_cursor;
  }
  return out;
}

function rtToMd(richText = []) {
  return richText.map(rt => {
    let txt = rt.plain_text ?? '';
    const ann = rt.annotations ?? {};
    if (ann.code) txt = `\`${txt}\``;
    if (ann.bold) txt = `**${txt}**`;
    if (ann.italic) txt = `*${txt}*`;
    if (ann.strikethrough) txt = `~~${txt}~~`;
    const href = rt.href;
    if (href) txt = `[${txt}](${href})`;
    return txt;
  }).join('');
}

function blockToMd(block, indent = 0) {
  const t = block.type;
  const pre = '  '.repeat(indent);
  if (!t) return '';

  if (t.startsWith('heading_')) {
    const lvl = Number(t.split('_')[1]);
    return `${'#'.repeat(lvl)} ${rtToMd(block[t]?.rich_text)}\n`;
  }
  if (t === 'paragraph') {
    const txt = rtToMd(block.paragraph?.rich_text);
    return txt.trim() ? `${pre}${txt}\n\n` : '\n';
  }
  if (t === 'bulleted_list_item' || t === 'numbered_list_item') {
    const marker = t === 'bulleted_list_item' ? '-' : '1.';
    const txt = rtToMd(block[t]?.rich_text);
    return `${pre}${marker} ${txt}\n`;
  }
  if (t === 'to_do') {
    const checked = !!block.to_do?.checked;
    const txt = rtToMd(block.to_do?.rich_text);
    return `${pre}- [${checked ? 'x' : ' '}] ${txt}\n`;
  }
  if (t === 'quote') {
    const txt = rtToMd(block.quote?.rich_text);
    return `> ${txt}\n\n`;
  }
  if (t === 'code') {
    const lang = block.code?.language ?? '';
    const txt = rtToMd(block.code?.rich_text);
    return `\n\`\`\`${lang}\n${txt}\n\`\`\`\n\n`;
  }
  if (t === 'child_page') {
    const title = block.child_page?.title ?? '';
    return `- ${title}\n`;
  }
  // fallback
  const inner = block[t];
  if (inner?.rich_text) {
    const txt = rtToMd(inner.rich_text);
    return txt.trim() ? `${pre}${txt}\n\n` : '';
  }
  return '';
}

async function exportPage(pageId, title, outPath) {
  const blocks = await getAllChildren(pageId);
  let md = `# ${title}\n\n`;
  for (const b of blocks) {
    md += blockToMd(b);
    if (b.has_children && b.type !== 'child_page') {
      const kids = await getAllChildren(b.id);
      for (const k of kids) md += blockToMd(k, 1);
      md += '\n';
    }
  }
  await fs.mkdir(path.dirname(outPath), { recursive: true });
  await fs.writeFile(outPath, md.trimEnd() + '\n', 'utf8');
}

const stdin = await new Promise((resolve, reject) => {
  let data = '';
  process.stdin.setEncoding('utf8');
  process.stdin.on('data', (c) => (data += c));
  process.stdin.on('end', () => resolve(data));
  process.stdin.on('error', reject);
});

const mapping = JSON.parse(stdin);
for (const [pageId, meta] of Object.entries(mapping)) {
  await exportPage(pageId, meta.title, meta.path);
}
