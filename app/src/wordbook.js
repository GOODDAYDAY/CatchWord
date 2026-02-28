import {invoke} from '@tauri-apps/api/core';

const searchEl = document.getElementById('search');
const filterEl = document.getElementById('filter');
const sortEl = document.getElementById('sort');
const listEl = document.getElementById('word-list');
const countEl = document.getElementById('word-count');

let allWords = [];

async function loadWords() {
    allWords = await invoke('list_words');
    renderList();
}

function renderList() {
    const query = searchEl.value.trim().toLowerCase();
    const filter = filterEl.value;
    const sort = sortEl.value;

    let words = allWords.filter(w => {
        if (query && !w.word.includes(query) && !w.translation.includes(query)) return false;
        if (filter === 'favorited' && !w.favorited) return false;
        if (filter === 'mastered' && !w.mastered) return false;
        if (filter === 'unmastered' && w.mastered) return false;
        return true;
    });

    words.sort((a, b) => {
        switch (sort) {
            case 'latest':
                return b.last_query_time.localeCompare(a.last_query_time);
            case 'oldest':
                return a.query_time.localeCompare(b.query_time);
            case 'alpha':
                return a.word.localeCompare(b.word);
            case 'count':
                return b.query_count - a.query_count;
            default:
                return 0;
        }
    });

    countEl.textContent = `共 ${words.length} 个单词`;

    if (words.length === 0) {
        listEl.innerHTML = '<div class="empty-state">暂无单词</div>';
        return;
    }

    listEl.innerHTML = words.map(w => `
        <div class="word-card" data-word="${escapeAttr(w.word)}">
            <div class="card-header">
                <div class="card-word">${escapeHtml(w.word)}</div>
                <div class="card-badges">
                    ${w.favorited ? '<span class="badge badge-fav">收藏</span>' : ''}
                    ${w.mastered ? '<span class="badge badge-mastered">已掌握</span>' : ''}
                    <span class="badge badge-count">查询 ${w.query_count} 次</span>
                </div>
            </div>
            <div class="card-phonetic">${w.phonetic ? '/' + escapeHtml(w.phonetic) + '/' : ''}</div>
            <div class="card-translation" data-word="${escapeAttr(w.word)}">${escapeHtml(w.translation)}</div>
            ${w.examples.length > 0 ? '<div class="card-examples">' + w.examples.slice(0, 2).map(e => '<div class="card-example">' + escapeHtml(e) + '</div>').join('') + '</div>' : ''}
            <div class="card-actions">
                <button class="action-btn fav-action ${w.favorited ? 'active' : ''}" data-action="fav" data-word="${escapeAttr(w.word)}" title="${w.favorited ? '取消收藏' : '收藏'}">
                    <svg viewBox="0 0 24 24" width="14" height="14"><path d="M12 21.35l-1.45-1.32C5.4 15.36 2 12.28 2 8.5 2 5.42 4.42 3 7.5 3c1.74 0 3.41.81 4.5 2.09C13.09 3.81 14.76 3 16.5 3 19.58 3 22 5.42 22 8.5c0 3.78-3.4 6.86-8.55 11.54L12 21.35z"/></svg>
                </button>
                <button class="action-btn mastered-action ${w.mastered ? 'active' : ''}" data-action="mastered" data-word="${escapeAttr(w.word)}" title="${w.mastered ? '标记未掌握' : '标记已掌握'}">
                    <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2"><polyline points="20 6 9 17 4 12"></polyline></svg>
                </button>
                <button class="action-btn delete-action" data-action="delete" data-word="${escapeAttr(w.word)}" title="删除">
                    <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2"><polyline points="3 6 5 6 21 6"></polyline><path d="M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a2 2 0 012-2h4a2 2 0 012 2v2"></path></svg>
                </button>
            </div>
        </div>
    `).join('');
}

// Event delegation for card actions
listEl.addEventListener('click', async (e) => {
    const btn = e.target.closest('[data-action]');
    if (!btn) return;

    const action = btn.dataset.action;
    const word = btn.dataset.word;

    if (action === 'fav') {
        const entry = allWords.find(w => w.word === word);
        if (!entry) return;
        const newVal = !entry.favorited;
        await invoke('set_favorited', {word, favorited: newVal});
        entry.favorited = newVal;
        renderList();
    } else if (action === 'mastered') {
        const entry = allWords.find(w => w.word === word);
        if (!entry) return;
        const newVal = !entry.mastered;
        await invoke('set_mastered', {word, mastered: newVal});
        entry.mastered = newVal;
        renderList();
    } else if (action === 'delete') {
        if (!confirm(`确定删除「${word}」吗？`)) return;
        await invoke('delete_word', {word});
        allWords = allWords.filter(w => w.word !== word);
        renderList();
    }
});

// Double-click on translation to edit inline
listEl.addEventListener('dblclick', (e) => {
    const translationEl = e.target.closest('.card-translation');
    if (!translationEl || translationEl.querySelector('input')) return;

    const word = translationEl.dataset.word;
    const entry = allWords.find(w => w.word === word);
    if (!entry) return;

    const original = entry.translation;
    const input = document.createElement('input');
    input.type = 'text';
    input.className = 'inline-edit';
    input.value = original;
    translationEl.textContent = '';
    translationEl.appendChild(input);
    input.focus();
    input.select();

    const commit = async () => {
        const newVal = input.value.trim();
        if (newVal && newVal !== original) {
            await invoke('update_translation', {word, translation: newVal});
            entry.translation = newVal;
        }
        renderList();
    };

    input.addEventListener('blur', commit);
    input.addEventListener('keydown', (ev) => {
        if (ev.key === 'Enter') input.blur();
        if (ev.key === 'Escape') {
            input.value = original;
            input.blur();
        }
    });
});

searchEl.addEventListener('input', renderList);
filterEl.addEventListener('change', renderList);
sortEl.addEventListener('change', renderList);

function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

function escapeAttr(text) {
    return text.replace(/&/g, '&amp;').replace(/"/g, '&quot;');
}

loadWords();
