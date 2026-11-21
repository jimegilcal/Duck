import type { Skin } from '../core/types';

export function DuckAccessories({ skin }: { skin: Skin }) {
  switch (skin) {
    case 'glasses':
      return <group position={[0,.85,.655]}>
        {[-1,1].map(side => <mesh key={side} position={[side*.245,0,0]}>
          <torusGeometry args={[.14,.022,8,32]}/>
          <meshStandardMaterial color="#374639"/>
        </mesh>)}
        <mesh><boxGeometry args={[.2,.025,.025]}/><meshStandardMaterial color="#374639"/></mesh>
      </group>;
    case 'party':
      return <group position={[.08,1.23,.11]} rotation={[.02,0,-.23]}>
        <mesh position={[0,.19,0]}><coneGeometry args={[.26,.5,24]}/><meshStandardMaterial color="#c25ac5"/></mesh>
        <mesh rotation={[Math.PI/2,0,0]}><torusGeometry args={[.243,.035,8,32]}/><meshStandardMaterial color="#f5b43f"/></mesh>
        <mesh position={[0,.45,0]}><sphereGeometry args={[.085,18,12]}/><meshStandardMaterial color="#ffd45a"/></mesh>
        <mesh position={[.07,.15,.16]}><sphereGeometry args={[.038,12,8]}/><meshStandardMaterial color="#ffe395"/></mesh>
        <mesh position={[-.12,.08,.19]}><sphereGeometry args={[.034,12,8]}/><meshStandardMaterial color="#ffddec"/></mesh>
      </group>;
    case 'bow':
      return <group position={[0,.19,.57]}>
        {[-1,1].map(side => <mesh key={side} position={[side*.15,0,0]} rotation={[0,0,side*.18]} scale={[.19,.11,.065]}>
          <sphereGeometry args={[1,20,14]}/><meshStandardMaterial color="#ec4e87"/>
        </mesh>)}
        <mesh position={[0,0,.045]} scale={[.085,.077,.06]}><sphereGeometry args={[1,16,12]}/><meshStandardMaterial color="#ff88ad"/></mesh>
      </group>;
    case 'frog':
      return <group position={[0,1.13,.13]}>
        <mesh scale={[.51,.23,.45]}><sphereGeometry args={[1,24,16]}/><meshStandardMaterial color="#66b967"/></mesh>
        {[-1,1].map(side => <group key={side} position={[side*.255,.19,.14]}>
          <mesh scale={[.148,.16,.135]}><sphereGeometry args={[1,20,14]}/><meshStandardMaterial color="#66b967"/></mesh>
          <mesh position={[0,.018,.107]} scale={[.086,.092,.038]}><sphereGeometry args={[1,16,12]}/><meshStandardMaterial color="#fff8ce"/></mesh>
          <mesh position={[side*.011,.015,.138]} scale={[.036,.047,.022]}><sphereGeometry args={[1,14,10]}/><meshStandardMaterial color="#26382d"/></mesh>
        </group>)}
      </group>;
    case 'headphones':
      return <group position={[0,.74,.15]}>
        <mesh><torusGeometry args={[.57,.047,10,36,Math.PI]}/><meshStandardMaterial color="#7855a9"/></mesh>
        {[-1,1].map(side => <group key={side} position={[side*.56,-.02,0]}>
          <mesh scale={[.103,.21,.202]}><sphereGeometry args={[1,20,16]}/><meshStandardMaterial color="#6545a0"/></mesh>
          <mesh position={[side*.076,0,0]} scale={[.052,.161,.16]}><sphereGeometry args={[1,20,14]}/><meshStandardMaterial color="#b18de5"/></mesh>
        </group>)}
      </group>;
    default:
      return null;
  }
}
