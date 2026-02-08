import os, sys, json
from urllib.request import Request, urlopen

NOTION_KEY = os.environ.get('NOTION_KEY')
NOTION_VER = '2025-09-03'

def api(path):
    return f"https://api.notion.com/v1{path}"

def http(method, path, data=None):
    url = api(path)
    headers = {
        'Authorization': f'Bearer {NOTION_KEY}',
        'Notion-Version': NOTION_VER,
        'Content-Type': 'application/json',
    }
    body = None
    if data is not None:
        body = json.dumps(data).encode('utf-8')
    req = Request(url, data=body, method=method, headers=headers)
    with urlopen(req) as r:
        return json.loads(r.read().decode('utf-8'))

def get_all_children(block_id, page_size=100):
    out = []
    cursor = None
    while True:
        qs = f"?page_size={page_size}" + (f"&start_cursor={cursor}" if cursor else "")
        res = http('GET', f"/blocks/{block_id}/children{qs}")
        out.extend(res.get('results', []))
        if not res.get('has_more'):
            break
        cursor = res.get('next_cursor')
    return out

def rt_to_md(rich_text):
    parts = []
    for rt in rich_text or []:
        txt = rt.get('plain_text','')
        ann = rt.get('annotations', {})
        if ann.get('code'):
            txt = f"`{txt}`"
        if ann.get('bold'):
            txt = f"**{txt}**"
        if ann.get('italic'):
            txt = f"*{txt}*"
        if ann.get('strikethrough'):
            txt = f"~~{txt}~~"
        href = rt.get('href')
        if href:
            txt = f"[{txt}]({href})"
        parts.append(txt)
    return ''.join(parts)

def block_to_md(block, indent=0):
    t = block.get('type')
    pre = '  '*indent

    if t.startswith('heading_'):
        lvl = int(t.split('_')[1])
        return f"{'#'*lvl} {rt_to_md(block[t].get('rich_text'))}\n"
    if t == 'paragraph':
        txt = rt_to_md(block[t].get('rich_text'))
        return (f"{pre}{txt}\n\n" if txt.strip() else "\n")
    if t in ('bulleted_list_item','numbered_list_item'):
        marker = '-' if t=='bulleted_list_item' else '1.'
        txt = rt_to_md(block[t].get('rich_text'))
        return f"{pre}{marker} {txt}\n"
    if t == 'to_do':
        checked = block[t].get('checked', False)
        box = 'x' if checked else ' '
        txt = rt_to_md(block[t].get('rich_text'))
        return f"{pre}- [{box}] {txt}\n"
    if t == 'quote':
        txt = rt_to_md(block[t].get('rich_text'))
        return f"> {txt}\n\n"
    if t == 'code':
        lang = block[t].get('language','')
        txt = rt_to_md(block[t].get('rich_text'))
        return f"```{lang}\n{txt}\n```\n\n"
    if t == 'child_page':
        title = block[t].get('title','')
        return f"- {title}\n"
    # fallback
    inner = block.get(t, {})
    if isinstance(inner, dict) and 'rich_text' in inner:
        txt = rt_to_md(inner.get('rich_text'))
        return f"{pre}{txt}\n\n" if txt.strip() else ''
    return ''

def export_page(page_id, title, out_path):
    blocks = get_all_children(page_id)
    md = [f"# {title}\n\n"]
    for b in blocks:
        md.append(block_to_md(b))
        if b.get('has_children') and b.get('type') not in ('child_page',):
            kids = get_all_children(b['id'])
            for k in kids:
                md.append(block_to_md(k, indent=1))
            md.append("\n")
    os.makedirs(os.path.dirname(out_path), exist_ok=True)
    with open(out_path,'w',encoding='utf-8') as f:
        f.write(''.join(md).rstrip()+"\n")

def main():
    if not NOTION_KEY:
        print('Missing NOTION_KEY', file=sys.stderr)
        sys.exit(2)
    mapping = json.loads(sys.stdin.read())
    for page_id, meta in mapping.items():
        export_page(page_id, meta['title'], meta['path'])

if __name__ == '__main__':
    main()
