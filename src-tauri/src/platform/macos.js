// Arguments are JSON data, never interpolated into executable script source.
ObjC.import('ApplicationServices');
function run(argv) {
    var r = JSON.parse(argv[0]);
    var se = Application('System Events');
    if (r.op === 'check') {
        if (!$.AXIsProcessTrusted()) {
            throw new Error('Allow Agent Pulse (and osascript if requested) in Privacy & Security > Accessibility and Automation > System Events, then retry.');
        }
        se.applicationProcesses.length;
        return 'ok';
    }
    if (r.op === 'list') {
        var result = [];
        se.applicationProcesses.whose({backgroundOnly:false})().forEach(function(p) {
            try {
                // One target per window so the Rust resolver reports ambiguity
                // for multiple matching windows, including a persisted PID hint.
                var pid = p.unixId();
                p.windows().forEach(function(w) {
                    result.push({hwnd:pid, process_id:pid, process_name:p.name(),
                        executable_path:null, title:w.name(), is_visible:true});
                });
            } catch (_) { /* inaccessible processes are not targetable */ }
        });
        return JSON.stringify(result);
    }
    var processes = se.applicationProcesses.whose({unixId:r.pid})();
    if (processes.length !== 1) return 'LOST_FOCUS';
    var p = processes[0];
    function guard() {
        if (p.windows().length !== 1) return 'AMBIGUOUS';
        if (!p.frontmost()) return 'LOST_FOCUS';
        // Shift, Control, Option, Command. Leave user-held keys untouched.
        if ((Number($.CGEventSourceFlagsState(1)) & 0x1e0000) !== 0) return 'MODIFIER_HELD';
        return null;
    }
    if (r.op === 'activate') {
        if (p.windows().length !== 1) return 'AMBIGUOUS';
        p.visible = true;
        try { p.windows[0].attributes.byName('AXMinimized').value = false; } catch (_) {}
        p.frontmost = true;
        for (var n = 0; n < 10 && !p.frontmost(); n++) delay(0.05);
        return p.frontmost() ? 'ok' : 'LOST_FOCUS';
    }
    if (r.op === 'foreground') return String(p.frontmost() && p.windows().length === 1);
    var error = guard();
    if (error) return error;
    if (r.op === 'text') {
        // Preserve surrogate pairs; verify focus before every character.
        for (var i = 0; i < r.value.length; i++) {
            error = guard(); if (error) return error;
            var c = r.value.charAt(i);
            var code = r.value.charCodeAt(i);
            if (code >= 0xd800 && code <= 0xdbff && i + 1 < r.value.length) c += r.value.charAt(++i);
            se.keystroke(c);
        }
    } else if (r.op === 'key') {
        if (r.value.code !== undefined) se.keyCode(r.value.code, {using:r.value.modifiers});
        else se.keystroke(r.value.text, {using:r.value.modifiers});
    }
    return 'ok';
}
