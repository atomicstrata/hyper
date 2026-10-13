import {createHyperViewer} from '../index.mjs';
const backend=new URL(location.href).searchParams.get('backend') ?? 'auto';
for(const [title,fixture] of [['Coauthorship','sample.json'],['HIF identities','sample.hif.json']]) {
  const section=document.getElementById('viewer-template').content.cloneNode(true).querySelector('section');
  document.getElementById('viewers').append(section);section.querySelector('h2').textContent=title;
  const status=section.querySelector('.status');let viewer;
  const errorBox=section.querySelector('.error');
  const report=error=>{errorBox.hidden=false;errorBox.textContent=error.message ?? String(error);};
  const enable=value=>{for(const control of section.querySelectorAll('button,select,input'))control.disabled=!value;};
  async function mount() {
    enable(false);errorBox.hidden=true;status.textContent='Loading graph…';
    section.querySelector('.pause').textContent='Pause layout';
    section.querySelector('.projection').value='bipartite';
    try {
    viewer=await createHyperViewer({canvas:section.querySelector('canvas'),backend});
    section.querySelector('.backend').textContent=viewer.backend;
    viewer.on('selectionchange',selection=>{section.querySelector('.selection').textContent=JSON.stringify(selection,null,2);});
    viewer.on('statechange',state=>{status.textContent=`${state.graph.vertices} vertices · ${state.graph.hyperedges} hyperedges · ${state.graph.iterations} layout steps`;});
    viewer.on('error',report);
    await viewer.loadJson(await (await fetch(fixture)).text());
    const state=viewer.getState();status.textContent=`${state.graph.vertices} vertices · ${state.graph.hyperedges} hyperedges · ${state.graph.iterations} layout steps`;
    section.querySelector('.lifecycle').textContent='Dispose';enable(true);
    } catch(error) {
      if(viewer)await viewer.dispose().catch(()=>{});viewer=null;
      section.querySelector('.lifecycle').textContent='Retry';section.querySelector('.lifecycle').disabled=false;report(error);
    }
  }
  const action=fn=>()=>Promise.resolve().then(()=>{errorBox.hidden=true;return fn();}).catch(report);
  section.querySelector('.frame').onclick=action(()=>viewer.frame());
  section.querySelector('.projection').onchange=action(()=>viewer.setProjection(section.querySelector('.projection').value));
  section.querySelector('.pause').onclick=action(async()=>{
    const running=!viewer.getState().settings.layoutRunning;
    await viewer.setSettings({layoutRunning:running});section.querySelector('.pause').textContent=running?'Pause layout':'Resume layout';
  });
  section.querySelector('input').onchange=action(async()=>{const file=section.querySelector('input').files[0];if(file)await viewer.loadJson(await file.text());});
  section.querySelector('.lifecycle').onclick=action(async()=>{
    if(viewer){enable(false);await viewer.dispose();viewer=null;section.querySelector('.lifecycle').textContent='Remount';section.querySelector('.lifecycle').disabled=false;status.textContent='Viewer disposed';}
    else {const old=section.querySelector('canvas');old.replaceWith(old.cloneNode(false));await mount();}
  });
  mount().catch(report);
}
