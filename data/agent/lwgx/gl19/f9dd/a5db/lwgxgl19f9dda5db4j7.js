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

const ADDENDUM = `Never try to scan the whole filesystem or even the working directory-- there could be millions of files with many of them attached via the network over slow connections. Prefer \`scratch.skills.find\` over \`evaluate_rust\`+\`read_dir\` and over shelling out to \`grep\`/\`find\`. If you know the name, resolve it; if you have the id, compute the path. **Never scan to discover what the store indexes.**
UNDER NO CIRCUMSTANCES SHOULD YOU EVER EDIT FILES IN THE ./data FOLDER DIRECTLY. ALWAYS USE THE APPROPRIATE COMMANDS TO MODIFY THE DATA STORE.`;

Object.assign(me, { ADDENDUM });
