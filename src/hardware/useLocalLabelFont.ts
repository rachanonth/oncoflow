import { useEffect, useState } from "react";

export function useLocalLabelFont(fontName: string | null | undefined) {
  const [loaded, setLoaded] = useState<{ name: string; family: string | null }>({ name: "", family: null });
  useEffect(() => {
    if (!fontName) return;
    let active = true;
    const family = `OncoFlowLabel-${fontName}`;
    const face = new FontFace(family, `local(${JSON.stringify(fontName)})`);
    void face.load().then(() => {
      if (!active) return;
      document.fonts.add(face);
      setLoaded({ name: fontName, family });
    }).catch(() => {
      if (active) setLoaded({ name: fontName, family: null });
    });
    return () => { active = false; document.fonts.delete(face); };
  }, [fontName]);
  const ready = !!fontName && loaded.name === fontName;
  return {
    fontFamily: ready && loaded.family ? JSON.stringify(loaded.family) : undefined,
    error: ready && !loaded.family ? "ไม่สามารถแสดงตัวอย่างด้วยฟอนต์ที่เลือกได้ กรุณาตรวจสอบฟอนต์ในเครื่อง" : null,
  };
}
