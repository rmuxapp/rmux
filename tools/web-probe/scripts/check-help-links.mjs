import assert from 'node:assert/strict';
import {test} from 'node:test';
import {openHelpLink} from '../engine.js';

test('successful help navigation removes opener access and leaves the demo alone', () => {
  const calls = [];
  const tab = {opener: {}, location: {replace(url) { calls.push(url); }}};
  openHelpLink('https://rmux.app', 'Website', {
    open(url, target) { assert.equal(url, 'about:blank'); assert.equal(target, '_blank'); return tab; },
  }, {getElementById() { throw new Error('successful opening must not show a dialog'); }});
  assert.equal(tab.opener, null);
  assert.deepEqual(calls, ['https://rmux.app']);
});

for (const throws of [false, true]) test(`blocked help keeps an accessible link and restores focus (throws=${throws})`, () => {
  let focused;
  class Element extends EventTarget {
    constructor(tag) { super(); this.tag = tag; this.children = []; this.isConnected = true; }
    setAttribute(name, value) { this[name] = value; }
    append(...children) { this.children.push(...children); }
    showModal() { this.modal = true; }
    close() { this.dispatchEvent(new Event('close')); }
    remove() { this.isConnected = false; }
    focus() { focused = this; }
  }
  const origin = new Element('input');
  const doc = {activeElement: origin, body: new Element('body'),
    createElement: tag => new Element(tag), getElementById: () => null};
  openHelpLink('https://rmux.app', 'Website', {open() { if (throws) throw new Error('blocked'); return null; }}, doc);
  const dialog = doc.body.children[0];
  const [title, explanation, link, close] = dialog.children;
  assert.equal(dialog.modal, true);
  assert.equal(dialog['aria-labelledby'], title.id);
  assert.equal(title.textContent, 'Website');
  assert.match(explanation.textContent, /did not open/);
  assert.equal(link.href, 'https://rmux.app');
  assert.equal(link.rel, 'noopener noreferrer');
  assert.equal(link.target, '_blank');
  assert.equal(focused, link);
  close.dispatchEvent(new Event('click'));
  assert.equal(dialog.isConnected, false);
  assert.equal(focused, origin);
});
