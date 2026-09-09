// agentprompt.js — the OWNER'S system-prompt addendum. Whatever ADDENDUM
// contains is appended to the agent's system prompt on every ask (under an
// "OWNER ADDENDUM" heading), so prompt experiments happen HERE — edit
// the `agentprompt` library control's js facet in the workbench; every
// change is journaled and takes effect on the next ask, no reinstall. When something
// proves out, promote it into agentloop.js's base prompt.
//
// Keep it a plain template literal. Empty string = no addendum.
//
// LIBRARY control — headless: the api rides this control's own element
// (zero-globals doctrine; el.api IS this constructor object). Consumers
// mount it as a hidden data-control child div and read the element's api.

var me = this;
var ME = document.getElementById(me.UUID);

const ADDENDUM = ``;

Object.assign(me, { ADDENDUM });
