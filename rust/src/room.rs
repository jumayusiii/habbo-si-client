use serde_json::Value;

/// Reads the current room out of the hotel client. This is a direct port of the C#
/// `RoomProbe`: cheap DOM selectors and the document title answer for nearly every room,
/// and walking the React fiber tree is only the fallback.
pub const ROOM_JS: &str = r#"(function(){try{
function clean(s){if(!s)return '';s=String(s).replace(/\s+/g,' ').trim();if(s.length<2||s.length>90)return '';return s;}
function generic(n){n=(n||'').toLowerCase().replace(/\s+/g,' ').trim();
return n===''||n==='habbo'||n==='habbo.si'||n==='einchecken'||n==='login'||n==='willkommen'||n==='registrieren'||n==='hotel'||n==='client'||n==='room info'||n==='rauminfo'||n==='info'||n==='settings'||n==='navigator'||n==='catalog'||n==='inventory'||n==='menu'||n==='owner'||n==='besitzer'||n==='description'||n==='beschreibung';}
function ok(s){var v=clean(s);return v&&!generic(v)?v:'';}
function peekProps(o){
if(!o||typeof o!=='object')return '';
try{
var ss=o.roomSession;
if(ss){var v=ok(ss.roomName);if(v)return v;if(ss.roomData)return ok(ss.roomData.name);}
}catch(e1){}
return '';
}
function peekState(o){
if(!o||typeof o!=='object')return '';
try{
var v=ok(o.roomName);if(v)return v;
if(o.roomData)return ok(o.roomData.name);
}catch(e1){}
return '';
}
function fromFiber(){
var el=document.getElementById('root')||document.body;if(!el)return '';
var k=Object.keys(el).filter(function(x){return x.indexOf('__reactContainer$')===0;})[0];
var f=k?el[k]:(el._reactRootContainer?el._reactRootContainer._internalRoot.current:null);
if(!f)return '';
var stack=[f],seen=0;
while(stack.length&&seen<9000){
var n=stack.pop();seen++;
var v=peekProps(n.memoizedProps);
if(!v&&n.memoizedState){var h=n.memoizedState,c=0;while(h&&c<24){v=peekState(h.memoizedState);if(v)break;h=h.next;c++;}}
if(v)return v;
if(n.sibling)stack.push(n.sibling);
if(n.child)stack.push(n.child);
}
return '';
}
var room='';
var sels=['[data-room-name]','[data-roomname]','.room-name','.roomName','[class*="room-name" i]','[class*="roomName" i]','[class*="nitro-room-tools-info" i]','[class*="room-tools-info" i]','[class*="room-info" i]','[class*="roominfo" i]','[id*="room-info" i]'];
for(var i=0;i<sels.length&&!room;i++){
var els=document.querySelectorAll(sels[i]);
for(var j=0;j<els.length&&j<8;j++){
var el=els[j],a=ok(el.getAttribute&&el.getAttribute('data-room-name'));
if(a){room=a;break;}
var txt=(el.innerText||'').split('\n');
for(var q=0;q<txt.length;q++){var t=ok(txt[q]);if(t){room=t;break;}}
if(room)break;
}}
var fiber='';
try{if(!room){var g=window.Nitro||window.nitro;if(g){var ins=g.instance||g;var ss=ins.roomSession||(ins.roomSessionManager&&ins.roomSessionManager.getSession?ins.roomSessionManager.getSession(-1):null);if(ss)room=ok(ss.roomName||(ss.roomData&&ss.roomData.name)||'');}}}catch(e2){}
try{if(!room&&window.GetNitroInstance){var ins2=window.GetNitroInstance();var ss2=ins2&&ins2.roomSessionManager&&ins2.roomSessionManager.getSession?ins2.roomSessionManager.getSession(-1):null;if(ss2)room=ok(ss2.roomName||(ss2.roomData&&ss2.roomData.name)||'');}}catch(e3){}
var title=clean(document.title),titleRoom='';
var m=title.match(/^(.*?)\s*(?:[-–—|·]\s*)(?:Habbo|Habbo\.si|Hotel|Nitro)(?:\s.*)?$/i);
if(m&&m[1])titleRoom=ok(m[1]);
if(!room)fiber=fromFiber();
return JSON.stringify({room:room,fiber:fiber,title:title,titleRoom:titleRoom});
}catch(e){return '';}})()"#;

/// Diagnostics for `--probe`: the interesting globals and DOM nodes the hotel exposes.
pub const DIAG_JS: &str = r#"(function(){try{
function cls(el){try{return (el.className&&el.className.toString)?el.className.toString():'';}catch(e){return '';}}
var out={url:location.href,title:document.title,ready:document.readyState,keys:[],nodes:[]};
try{out.keys=Object.keys(window).filter(function(k){return /nitro|room|habbo|client|user|engine/i.test(k);}).slice(0,50);}catch(e){}
var all=document.querySelectorAll('*');
for(var i=0;i<all.length&&out.nodes.length<70;i++){
var el=all[i],c=cls(el),id=el.id||'';
if(!/room|user|name|info|header/i.test(c+' '+id))continue;
var t=(el.innerText||'').replace(/\s+/g,' ').trim();
if(!t||t.length>140)continue;
out.nodes.push({t:el.tagName,c:c.slice(0,100),i:id,x:t,n:el.children.length});
}
out.text=(document.body.innerText||'').replace(/\s+/g,' ').slice(0,1800);
return JSON.stringify(out);
}catch(e){return JSON.stringify({err:String(e)});}})()"#;

const GENERIC: &[&str] = &[
    "habbo",
    "habbo.si",
    "einchecken",
    "login",
    "willkommen",
    "registrieren",
    "hotel",
    "client",
    "room info",
    "rauminfo",
    "info",
    "settings",
    "navigator",
    "catalog",
    "inventory",
    "menu",
];

/// `evaluate_script` hands back the JSON encoding of the script's value. The probe
/// returns a JSON string, so the result arrives quoted twice.
pub fn unquote(raw: &str) -> String {
    serde_json::from_str::<String>(raw).unwrap_or_else(|_| raw.to_string())
}

pub fn is_generic(name: &str) -> bool {
    let name = name.trim().to_lowercase();
    GENERIC.contains(&name.as_str())
}

pub fn pick_room(raw: &str) -> Option<String> {
    if raw.trim().is_empty() {
        return None;
    }
    let value: Value = serde_json::from_str(raw).ok()?;
    for key in ["room", "fiber", "titleRoom"] {
        if let Some(text) = value.get(key).and_then(Value::as_str) {
            let text = text.trim();
            if !text.is_empty() && !is_generic(text) {
                return Some(text.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_first_useful_field() {
        let raw = r#"{"room":"","fiber":"Sunset Lounge","title":"x","titleRoom":"Fallback"}"#;
        assert_eq!(pick_room(raw).as_deref(), Some("Sunset Lounge"));
    }

    #[test]
    fn ignores_generic_titles() {
        assert_eq!(pick_room(r#"{"room":"Habbo","fiber":"","titleRoom":"Login"}"#), None);
        assert_eq!(pick_room(r#"{"room":"Suite 42"}"#).as_deref(), Some("Suite 42"));
    }

    #[test]
    fn unquotes_double_encoded_results() {
        assert_eq!(unquote("\"{\\\"room\\\":\\\"A\\\"}\""), r#"{"room":"A"}"#);
        assert_eq!(unquote("plain"), "plain");
    }
}
