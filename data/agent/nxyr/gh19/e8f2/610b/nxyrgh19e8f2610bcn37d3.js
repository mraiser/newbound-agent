var me = this;
var ME = document.getElementById(me.UUID);

me.ready = function() {
  var chatHistory = ME.querySelector('#chat-history');
  var myprompt = ME.querySelector('#myprompt');
  var sysprompt = ME.querySelector('#sysprompt');
  var sysPromptArea = ME.querySelector('#sys-prompt-area');
  var sysToggle = ME.querySelector('#sys-toggle');
  var sendBtn = ME.querySelector('#mybutton');

  // Toggle System Prompt
  sysToggle.addEventListener('click', function() {
    var visible = sysPromptArea.style.display !== 'none';
    sysPromptArea.style.display = visible ? 'none' : '';
    sysToggle.textContent = visible ? '⚙️ System Prompt' : '⚙️ Hide System Prompt';
  });

  // Auto-resize textarea
  myprompt.addEventListener('input', function() {
    this.style.height = 'auto';
    this.style.height = (this.scrollHeight) + 'px';
  });

  // Helper to add message to chat
  function addMessage(text, type) {
    var msgDiv = document.createElement('div');
    msgDiv.className = 'message ' + type;
    // If it's a JSON object, stringify it nicely, otherwise just text
    if (typeof text === 'object') {
      var code = document.createElement('code');
      code.textContent = JSON.stringify(text, null, 2);
      msgDiv.appendChild(code);
    } else {
      msgDiv.textContent = text;
    }
    chatHistory.appendChild(msgDiv);
    chatHistory.scrollTop = chatHistory.scrollHeight;
  }

  // Send Button Click
  sendBtn.addEventListener('click', function() {
    var userText = myprompt.value.trim();
    if (!userText) return;

    // Disable button and clear input
    sendBtn.disabled = true;
    sendBtn.textContent = '...';
    myprompt.value = '';
    myprompt.style.height = 'auto';

    // Add user message to UI immediately
    addMessage(userText, 'user');

    var sysText = sysprompt.value.trim();
    var sysPrompt = (sysText === "") ? null : sysText;

    // Call Backend
    send_tool_loop(userText, function(result) {
    //send_ask_llm(userText, sysPrompt, function(result) {
      sendBtn.disabled = false;
      sendBtn.textContent = 'Send';

      if (result.messages){
        for (var i in result.messages) {
          var mm = JSON.stringify(result.messages[i]);
          addMessage(mm, 'system');
        }
      }

      if (result.status === 'ok') {
        // Check if data is a string or object
        var responseText = result.msg || (result.data ? JSON.stringify(result.data) : "No response data");
        addMessage(responseText, 'system');
      } else {
        addMessage("Error: " + (result.msg || "Unknown error"), 'error');
      }
    });
  });

  // Allow Enter key to send (Shift+Enter for new line)
  myprompt.addEventListener('keydown', function(e) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      sendBtn.click();
    }
  });
};