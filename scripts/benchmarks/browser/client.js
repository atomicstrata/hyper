import ForceGraph3D from '3d-force-graph';
import cytoscape from 'cytoscape';
import Sigma from 'sigma';
import Graph from 'graphology';

const WIDTH = 1920, HEIGHT = 1080;
const host = document.getElementById('view');
let adapter, sourceScene, framing;
const color = n => n.role === 'edge' || n.id.startsWith('e:') ? '#ef9866' : '#5cbded';
const endpoint = p => typeof p === 'object' ? p.id : p;
const copy = x => structuredClone(x);
function bounds(nodes) {
  const xs = nodes.map(n => n.x), ys = nodes.map(n => n.y), zs = nodes.map(n => n.z);
  const extent = a => [Math.min(...a), Math.max(...a)];
  const [xmin,xmax]=extent(xs), [ymin,ymax]=extent(ys), [zmin,zmax]=extent(zs);
  return {x:(xmin+xmax)/2,y:(ymin+ymax)/2,z:(zmin+zmax)/2,
    span:Math.max(xmax-xmin,ymax-ymin,zmax-zmin,1),
    zoom:Math.min((WIDTH-80)/Math.max(xmax-xmin,1),(HEIGHT-80)/Math.max(ymax-ymin,1))};
}
function topology(nodes,links) {
  return {nodes:nodes.map(n=>({id:n.id,x:n.x,y:n.y,z:n.z})).sort((a,b)=>a.id.localeCompare(b.id)),
    links:links.map(l=>({id:l.id,source:endpoint(l.source),target:endpoint(l.target)})).sort((a,b)=>a.id.localeCompare(b.id))};
}
const raf = () => new Promise(resolve => requestAnimationFrame(resolve));
window.benchmark = {
  async setup(tool, scene) {
    sourceScene=copy(scene); framing=bounds(scene.nodes); host.innerHTML='';
    const nodes=copy(scene.nodes), links=copy(scene.links);
    if(tool==='3d-force-graph') {
      const graph=new ForceGraph3D(host,{controlType:'orbit',rendererConfig:{antialias:true,alpha:false}})
        .width(WIDTH).height(HEIGHT).backgroundColor('#080c14').showNavInfo(false)
        .nodeRelSize(2).nodeResolution(8).nodeVal(1).nodeLabel('').nodeColor(color)
        .linkWidth(0).linkOpacity(0.35).linkColor(()=> '#8fa9c4').linkLabel('')
        .enableNodeDrag(false).enablePointerInteraction(false).cooldownTicks(0)
        .graphData({nodes:nodes.map(n=>({...n,fx:n.x,fy:n.y,fz:n.z})),links});
      graph.renderer().setPixelRatio(1);
      await raf(); await raf(); graph.pauseAnimation();
      const camera=graph.camera(), distance=framing.span/(2*Math.tan(camera.fov*Math.PI/360))*1.18+framing.span/2;
      graph.cameraPosition({x:framing.x,y:framing.y,z:framing.z+distance},{x:framing.x,y:framing.y,z:framing.z},0);
      adapter={kind:'WebGL3D',counters(){return {render:{...graph.renderer().info.render},memory:{...graph.renderer().info.memory},meaning:'Three.js counters for last CPU render submission; not GPU duration or process RSS'};},cameraState(){return {position:camera.position.toArray(),fov:camera.fov,aspect:camera.aspect,near:camera.near,far:camera.far};}, render(angle){
        camera.position.set(framing.x+Math.sin(angle)*distance,framing.y,framing.z+Math.cos(angle)*distance);
        camera.lookAt(framing.x,framing.y,framing.z); graph.renderer().render(graph.scene(),camera);
      }, snapshot(){return topology(graph.graphData().nodes,graph.graphData().links);},
      update(updated){const old=new Map(graph.graphData().nodes.map(n=>[n.id,n]));
        graph.graphData({nodes:updated.nodes.map(n=>Object.assign(old.get(n.id)||{},n,{fx:n.x,fy:n.y,fz:n.z})),links:copy(updated.links)});
      },dispose(){graph._destructor();}};
    } else if(tool==='cytoscape') {
      const cy=cytoscape({container:host,layout:{name:'preset'},pixelRatio:1,minZoom:1e-9,maxZoom:1e9,
        style:[{selector:'node',style:{'width':4,'height':4,'background-color':n=>color(n.data()),'label':''}},
          {selector:'edge',style:{'width':1,'line-color':'#8fa9c4','opacity':0.35,'curve-style':'straight','label':''}}],
        elements:[...nodes.map(n=>({data:{...n},position:{x:n.x,y:-n.y}})),...links.map(l=>({data:{...l}}))],
        userZoomingEnabled:false,userPanningEnabled:false,boxSelectionEnabled:false});
      cy.zoom(framing.zoom); cy.pan({x:WIDTH/2-framing.x*framing.zoom,y:HEIGHT/2+framing.y*framing.zoom});
      adapter={kind:'Canvas2D',cameraState(){return {zoom:cy.zoom(),pan:cy.pan()};},render(angle){cy.pan({x:WIDTH/2-framing.x*framing.zoom+Math.sin(angle)*40,y:HEIGHT/2+framing.y*framing.zoom});cy.forceRender();},
        snapshot(){return topology(cy.nodes().map(n=>({...n.data(),x:n.position('x'),y:-n.position('y')})),cy.edges().map(e=>e.data()));},
        update(updated){cy.batch(()=>{for(const n of updated.nodes){cy.getElementById(n.id).data(n).position({x:n.x,y:-n.y});}
          const wanted=new Set(updated.links.map(l=>l.id));cy.edges().filter(e=>!wanted.has(e.id())).remove();
          for(const l of updated.links){const edge=cy.getElementById(l.id);if(edge.length)edge.move({source:l.source,target:l.target});else cy.add({data:{...l}});}});},dispose(){cy.destroy();}};
    } else if(tool==='sigma') {
      const graph=new Graph({type:'undirected',multi:true,allowSelfLoops:false});
      nodes.forEach(n=>graph.addNode(n.id,{...n,size:2,color:color(n),label:''}));
      links.forEach(l=>graph.addUndirectedEdgeWithKey(l.id,l.source,l.target,{size:1,color:'rgba(143,169,196,0.35)'}));
      const sigma=new Sigma(graph,host,{renderLabels:false,renderEdgeLabels:false,hideEdgesOnMove:false,hideLabelsOnMove:false,
        enableEdgeEvents:false,enableCameraRotation:false,stagePadding:40,zIndex:false});
      const camera=sigma.getCamera(); camera.setState({x:0.5,y:0.5,ratio:1,angle:0});
      adapter={kind:'WebGL2D',cameraState(){return camera.getState();},render(angle){camera.setState({x:0.5+Math.sin(angle)*0.025,y:0.5,ratio:1,angle:0});sigma.scheduleRender();},
        snapshot(){return topology(graph.mapNodes((id,a)=>({id,...a})),graph.mapEdges((id,a,source,target)=>({id,source,target})));},
        update(updated){for(const n of updated.nodes)graph.mergeNodeAttributes(n.id,n);
          const wanted=new Set(updated.links.map(l=>l.id));for(const id of graph.edges())if(!wanted.has(id))graph.dropEdge(id);
          for(const l of updated.links){if(!graph.hasEdge(l.id)||graph.source(l.id)!==l.source||graph.target(l.id)!==l.target){if(graph.hasEdge(l.id))graph.dropEdge(l.id);graph.addUndirectedEdgeWithKey(l.id,l.source,l.target,{size:1,color:'rgba(143,169,196,0.35)'});}}
          sigma.refresh();},dispose(){sigma.kill();}};
    } else throw new Error(`Unknown tool ${tool}`);
    adapter.render(0); await raf(); await raf();
    return {topology:adapter.snapshot(),kind:adapter.kind,framing,
      settings:{initialCameraState:adapter.cameraState(),width:WIDTH,height:HEIGHT,dpr:devicePixelRatio,labels:false,layout:'none; canonical fixed coordinates',lineOpacity:0.35,nodeSize2D:2,lineWidth2D:1,nodeRadius3D:2,nodeResolution3D:8,linkWidth3D:0,
        camera:tool==='3d-force-graph'?'perspective fit all coordinates, orbit around canonical center':'2D XY projection with Y upward, fitted initial viewport',
        commandTiming:tool==='cytoscape'?'public forceRender schedules Canvas redraw; submission only':tool==='sigma'?'public scheduleRender schedules cached WebGL redraw; submission only':'JavaScript render command; CPU only, excludes GPU completion'}};
  },
  snapshot(){return adapter.snapshot();},
  graphics(){return {contexts:window.__glContexts.map(gl=>{
    const debug=gl.getExtension('WEBGL_debug_renderer_info');
    return {version:gl.getParameter(gl.VERSION),renderer:debug?gl.getParameter(debug.UNMASKED_RENDERER_WEBGL):gl.getParameter(gl.RENDERER),softwareFallback:/swiftshader|llvmpipe|lavapipe|software/i.test(debug?gl.getParameter(debug.UNMASKED_RENDERER_WEBGL):gl.getParameter(gl.RENDERER)),vendor:debug?gl.getParameter(debug.UNMASKED_VENDOR_WEBGL):gl.getParameter(gl.VENDOR),
      timerQuerySupported:!!(gl.getExtension('EXT_disjoint_timer_query_webgl2')||gl.getExtension('EXT_disjoint_timer_query'))};
  }),gpuTiming:'not measured; WebGL timestamp extension availability reported only',gpuFinish:'not called; GPU synchronization would alter workload'};},
  async sample({mode,warmup=120,samples=300}) {
    const commandMs=[],rafIntervalMs=[],rafTimestampsMs=[];let previous;
    for(let frame=-warmup;frame<samples;frame++){
      const stamp=await raf(), angle=mode==='camera-motion'?Math.max(frame,0)*0.004:0;
      const start=performance.now();adapter.render(angle);const end=performance.now();
      if(frame>=0){commandMs.push(end-start);rafTimestampsMs.push(stamp);rafIntervalMs.push(stamp-previous);}previous=stamp;
    }
    return {commandMs,rafIntervalMs,rafTimestampsMs,mode,warmup,samples,rendererCounters:adapter.counters?.()||null,
      averageThroughputFPS:1000*samples/rafIntervalMs.reduce((a,b)=>a+b,0),
      meaning:'requestAnimationFrame callback intervals are wall cadence, not GPU duration or verified presented frames'};
  },
  async update(updated,measure=true){
    const start=performance.now();adapter.update(updated);const commandEnd=performance.now();adapter.render(0);
    await raf();await raf();const completion=performance.now();
    return {topology:adapter.snapshot(),...(measure?{commandMs:commandEnd-start,twoRafFeedbackSurrogateMs:completion-start}:{}),
      meaning:'script-triggered graph update followed by two rAF callbacks; not human input latency or compositor presentation'};
  },dispose(){adapter?.dispose();adapter=null;}
};
