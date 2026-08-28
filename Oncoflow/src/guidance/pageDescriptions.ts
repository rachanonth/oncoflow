export const PAGE_DESCRIPTIONS = [
  { key: "patients", title: "Patients", description: "ค้นหาและดูแลข้อมูลผู้ป่วย" },
  { key: "patient_form", title: "Patient form", description: "บันทึกและแก้ไขข้อมูลผู้ป่วย โดยคงรหัส HN และตรวจสอบข้อมูลซ้ำ" },
  { key: "drugs", title: "Drug master", description: "ดูแลข้อมูลยา การเตรียมยา ความปลอดภัย และการตั้งค่าคงคลัง" },
  { key: "drug_form", title: "Drug form", description: "บันทึกค่ากำหนดยาโดยไม่เรียกใช้สูตรคำนวณทางคลินิกในหน้านี้" },
  { key: "regimens", title: "Chemotherapy regimens", description: "ดูแลโครงสร้างสูตรยาและค่าการเตรียมยาดั้งเดิม" },
  { key: "regimen_form", title: "Regimen form", description: "เก็บรักษารหัสและค่าพฤติกรรมเดิม โดยจัดการขั้นตอนยาหลังจากบันทึกข้อมูลหลัก" },
  { key: "orders", title: "Orders", description: "ทบทวนคำสั่งยาเดิมและจัดการร่างคำสั่งยาใหม่" },
  { key: "order_form", title: "Order form", description: "ระบบประเมินกฎความปลอดภัยที่ยืนยันแล้วหลังบันทึกเพื่อให้เภสัชกรทบทวน และจะไม่ปรับค่าที่ป้อนโดยอัตโนมัติ" },
  { key: "preparation", title: "Preparation queue", description: "แสดงคำสั่งยาที่มีรายการซึ่งผ่านเกณฑ์การเตรียมยาที่กำหนดไว้" },
  { key: "inventory", title: "Inventory", description: "ตรวจสอบคงคลังสำหรับการเตรียมยาเคมีบำบัดและบันทึกการเคลื่อนไหวโดยผู้ใช้ที่ยืนยันตัวตน" },
  { key: "doctors", title: "Doctors", description: "ดูแลรายชื่อแพทย์สำหรับใช้เมื่อสร้างหรือแก้ไขคำสั่งยา" },
  { key: "wards", title: "Wards", description: "ดูแลรายชื่อหอผู้ป่วยสำหรับใช้เมื่อสร้างหรือแก้ไขคำสั่งยา" },
  { key: "diluents", title: "Diluents", description: "ดูแลชื่อตัวทำละลายและข้อมูลอ้างอิงปริมาตร" },
  { key: "routes", title: "Routes", description: "ดูแลชื่อวิถีการให้ยาที่ใช้กับยา สูตรยา และคำสั่งยา" },
  { key: "diagnoses", title: "Diagnosis", description: "ดูแลชื่อการวินิจฉัยที่ใช้กับข้อมูลผู้ป่วย" },
  { key: "account", title: "Account", description: "จัดการข้อมูลประจำตัวและรหัสผ่านของบัญชีที่กำลังเข้าสู่ระบบ" },
  { key: "general", title: "General settings", description: "กำหนดชื่อสถานพยาบาลที่ใช้แสดงบนเอกสาร" },
  { key: "users", title: "Users", description: "สร้างและจัดการบัญชีผู้ใช้" },
  { key: "guidance", title: "Guidance", description: "จัดการข้อความ Guidance เพิ่มเติมที่แสดงใต้คำอธิบายมาตรฐานของแต่ละหน้า" },
  { key: "hardware", title: "Label printer", description: "กำหนดค่า Windows RAW spooler สำหรับฉลากเตรียมยาเคมีบำบัด" },
  { key: "backup_restore", title: "Backup & restore", description: "สำรองและกู้คืนข้อมูลทั้งชุดพร้อมตรวจสอบความถูกต้อง" },
  { key: "diagnostics", title: "Diagnostics", description: "ตรวจสอบสถานะระบบ การสำรองข้อมูล และการพิมพ์ฉลากโดยไม่เปิดเผยข้อมูลผู้ป่วย" },
] as const;

export type PageKey = (typeof PAGE_DESCRIPTIONS)[number]["key"];

export function pageDescription(pageKey: PageKey) {
  return PAGE_DESCRIPTIONS.find((page) => page.key === pageKey)!;
}
