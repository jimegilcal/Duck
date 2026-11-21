import { Canvas, useFrame, useThree } from '@react-three/fiber';
import { useEffect, useRef } from 'react';
import { Group, WebGLRenderTarget, QuadraticBezierCurve3, Vector3 } from 'three';
import { invoke } from '@tauri-apps/api/core';

import type { Skin } from '../core/types';
import { DuckAccessories } from './DuckAccessories';
// Replace this component with a GLTF-backed model; animation and window code stay unchanged.
function DuckModel({ skin, pulse, animated }: { skin: Skin; pulse: number; animated: boolean }) {
  const group = useRef<Group>(null);
  const started = useRef(0);
  const { clock } = useThree();
  useEffect(() => { started.current = clock.elapsedTime; }, [pulse, clock]);
  useFrame(({ clock }) => {
    if (!group.current) return;
    const t = clock.elapsedTime;
    const dt = t - started.current;
    group.current.position.y = -0.48 + (animated ? Math.sin(t * 1.6) * 0.025 : 0) + (dt < .65 && animated ? Math.sin(dt / .65 * Math.PI) * .32 : 0);
    group.current.rotation.y = -.32 + (animated ? Math.sin(t * .7) * .045 : 0);
  });
  const smile = new QuadraticBezierCurve3(new Vector3(-.23,.55,.85), new Vector3(0,.43,.94), new Vector3(.23,.55,.85));
  const ball = (key:string, p:[number,number,number], s:[number,number,number], color:string, roughness=.36) => <mesh key={key} position={p} scale={s}><sphereGeometry args={[1, 32, 24]}/><meshStandardMaterial color={color} roughness={roughness}/></mesh>;
  return <group ref={group} position={[0,-.48,0]} rotation={[0,-.32,0]}>
    {ball('body',[0,0,0],[.72,.61,.58],'#ffd348')}
    {ball('head',[0,.72,.13],[.59,.57,.55],'#ffdc59')}
    {ball('beak',[0,.55,.65],[.34,.13,.28],'#ef922e')}
    <mesh><tubeGeometry args={[smile,24,.013,6,false]}/><meshStandardMaterial color="#a35820"/></mesh>
    {ball('cheek-l',[-.39,.65,.54],[.093,.045,.024],'#f5ad62')}
    {ball('cheek-r',[.39,.65,.54],[.093,.045,.024],'#f5ad62')}
    {ball('beak-bottom',[0,.48,.64],[.28,.055,.23],'#df7d23')}
    {ball('wing-left',[-.65,.08,.05],[.16,.35,.37],'#f4bc32')}
    {ball('wing-right',[.65,.08,.05],[.16,.35,.37],'#f7c43b')}
    {ball('tail',[0,.04,-.54],[.24,.26,.35],'#f6c337')}
    {[-1,1].map(side => <group key={side}>
      {ball('eye'+side,[side*.245,.85,.605],[.078,.09,.045],'#202722',.15)}
      {ball('spark'+side,[side*.245-.018,.881,.64],[.018,.021,.012],'#ffffff')}
      {ball('foot'+side,[side*.31,-.53,.27],[.24,.07,.3],'#ee962e')}
    </group>)}
    <DuckAccessories skin={skin}/>
  </group>;
}
function RenderController({active, animated, mask, skin}:{active:boolean; animated:boolean; mask:boolean; skin:Skin}) {
  const { gl, scene, camera, invalidate, size } = useThree();
  useEffect(() => {
    invalidate();
    if (!active || !animated) return;
    const id = setInterval(invalidate, 1000/24);
    return () => clearInterval(id);
  }, [active, animated, invalidate, skin]);
  useEffect(() => {
    if (!mask) return;
    const id = setTimeout(() => {
      const target = new WebGLRenderTarget(64,64);
      const pixels = new Uint8Array(64*64*4);
      const previous = gl.getRenderTarget();
      try {
        gl.setRenderTarget(target); gl.render(scene,camera);
        gl.readRenderTargetPixels(target,0,0,64,64,pixels);
        const alpha:number[]=[];
        for(let y=0;y<64;y++) for(let x=0;x<64;x++) {
          let hit=0;
          // Padding covers the subtle breathing animation and makes the small model easy to grab.
          for(let dy=-2;dy<=2;dy++) for(let dx=-2;dx<=2;dx++) {
            const px=x+dx, py=63-y+dy;
            if(px>=0 && px<64 && py>=0 && py<64 && pixels[(py*64+px)*4+3]>30) hit=1;
          }
          alpha.push(hit);
        }
        void invoke('set_hit_mask',{alpha}).catch(console.error);
      } finally { gl.setRenderTarget(previous); target.dispose(); invalidate(); }
    }, 800);
    return () => clearTimeout(id);
  },[gl,scene,camera,mask,size.width,size.height,invalidate,skin]);
  return null;
}
export function DuckScene({animated=true, active=true, pulse=0, skin='classic', mask=false}:{animated?:boolean;active?:boolean;pulse?:number;skin?:Skin;mask?:boolean}) {
  return <Canvas frameloop="demand" dpr={[1,1.5]} camera={{position:[0,1,5],fov:39}} gl={{alpha:true,antialias:true,powerPreference:'low-power'}}>
    <ambientLight intensity={1.7}/><directionalLight position={[-3,5,5]} intensity={3.2} color="#fff5dd"/><directionalLight position={[4,2,-3]} intensity={2} color="#e7f1ff"/>
    <DuckModel skin={skin} pulse={pulse} animated={animated}/><RenderController skin={skin} active={active} animated={animated} mask={mask}/>
  </Canvas>;
}
