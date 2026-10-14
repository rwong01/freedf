//! Original, redistributable fixture documents generated entirely by this project.
use super::*;
pub fn sample(pages: usize, forms: bool) -> Vec<u8> {
    let mut d = Document::with_version("1.7");
    let pages_id = d.new_object_id();
    let f = font(&mut d, false);
    let mut kids = vec![];
    let mut fields = vec![];
    for n in 0..pages {
        let content = format!(
            "BT /F1 24 Tf 60 730 Td (OpenPDF Desktop) Tj 0 -40 Td /F1 12 Tf (Page {} - Searchable text and signing fixture.) Tj 0 -25 Td (Select this sentence, highlight it, or add a comment.) Tj ET",
            n + 1
        );
        let stream = d.add_object(Stream::new(dictionary! {}, content.into_bytes()));
        let p = d.new_object_id();
        let mut page = dictionary! {"Type"=>"Page","Parent"=>pages_id,"MediaBox"=>vec![0.into(),0.into(),612.into(),792.into()],"Resources"=>dictionary!{"Font"=>dictionary!{"F1"=>f}},"Contents"=>stream};
        if forms && n == 0 {
            let mut annots = vec![];
            for (i, (name, ft, flags)) in [
                ("Name", "Tx", 0),
                ("Comments", "Tx", 4096),
                ("Agree", "Btn", 0),
                ("Country", "Ch", 131072),
                ("Readonly", "Tx", 1),
                ("Choices", "Ch", 2097152),
            ]
            .iter()
            .enumerate()
            {
                let y = 600. - i as f32 * 60.;
                let w = if *ft == "Btn" { 24. } else { 260. };
                let h = if *flags == 4096 { 48. } else { 24. };
                let mut field = dictionary! {"Type"=>"Annot","Subtype"=>"Widget","FT"=>*ft,"T"=>pdf_string(name),"Ff"=>*flags,"Rect"=>vec![60.into(),y.into(),(60.+w).into(),(y+h).into()],"P"=>p,"F"=>4,"DA"=>Object::string_literal("/Helv 12 Tf 0 g")};
                if *ft == "Ch" {
                    field.set(
                        "Opt",
                        vec![
                            pdf_string("Canada"),
                            pdf_string("France"),
                            pdf_string("Japan"),
                        ],
                    );
                }
                if *ft == "Btn" {
                    let ap = appearance(&mut d, w, h, format!("0.5 G 0 0 {w} {h} re S"), f, 1.);
                    field.set("AP", dictionary! {"N"=>dictionary!{"Off"=>ap,"Yes"=>ap}});
                    field.set("AS", "Off");
                } else {
                    let ap = appearance(&mut d, w, h, format!("0.85 G 0 0 {w} {h} re S"), f, 1.);
                    field.set("AP", dictionary! {"N"=>ap});
                }
                let id = d.add_object(field);
                fields.push(Object::Reference(id));
                annots.push(Object::Reference(id));
            }
            // A radio group with two widgets and one shared field value.
            let group = d.new_object_id();
            let mut radios = vec![];
            for (i, value) in ["One", "Two"].iter().enumerate() {
                let x = 350. + i as f32 * 45.;
                let ap = appearance(&mut d, 20., 20., "0 G 0 0 20 20 re S".into(), f, 1.);
                let mut normal = lopdf::Dictionary::new();
                normal.set("Off", ap);
                normal.set(*value, ap);
                let id=d.add_object(dictionary!{"Type"=>"Annot","Subtype"=>"Widget","Parent"=>group,"Rect"=>vec![x.into(),600.into(),(x+20.).into(),620.into()],"P"=>p,"AP"=>dictionary!{"N"=>normal},"AS"=>"Off"});
                radios.push(Object::Reference(id));
                annots.push(Object::Reference(id));
            }
            d.objects.insert(
                group,
                dictionary! {"FT"=>"Btn","T"=>pdf_string("Radio"),"Ff"=>32768,"Kids"=>radios}
                    .into(),
            );
            fields.push(group.into());
            page.set("Annots", annots);
        }
        d.objects.insert(p, page.into());
        kids.push(Object::Reference(p));
    }
    d.objects.insert(
        pages_id,
        dictionary! {"Type"=>"Pages","Kids"=>kids,"Count"=>pages as i64}.into(),
    );
    let mut root = dictionary! {"Type"=>"Catalog","Pages"=>pages_id};
    if forms {
        root.set("AcroForm",dictionary!{"Fields"=>fields,"DA"=>Object::string_literal("/Helv 12 Tf 0 g"),"DR"=>dictionary!{"Font"=>dictionary!{"Helv"=>f}}});
    }
    let root = d.add_object(root);
    d.trailer.set("Root", root);
    let mut out = vec![];
    d.save_to(&mut out).unwrap();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn annotations_forms_undo_reopen() {
        let mut d = DocumentModel::open(sample(3, true), None).unwrap();
        assert_eq!(d.pages.len(), 3);
        assert_eq!(d.fields.len(), 8);
        let name = d.fields.iter().find(|f| f.name == "Name").unwrap().id;
        let country = d.fields.iter().find(|f| f.name == "Country").unwrap().id;
        let checkbox = d.fields.iter().find(|f| f.name == "Agree").unwrap().id;
        let radio = d.fields.iter().find(|f| f.name == "Radio").unwrap().id;
        d.change(|e| {
            e.fields
                .insert(name, FieldValue::Text("Alice Smith".into()));
            e.fields
                .insert(country, FieldValue::Choice(vec!["Japan".into()]));
            e.fields.insert(checkbox, FieldValue::Button("Yes".into()));
            e.fields.insert(radio, FieldValue::Button("Two".into()));
        })
        .unwrap();
        let mut a = Annotation::new(
            1,
            0,
            AnnotationKind::Rectangle,
            Rect {
                left: 50.,
                bottom: 50.,
                right: 150.,
                top: 100.,
            },
        );
        a.text = "Comment".into();
        d.change(|e| e.annotations.push(a)).unwrap();
        assert!(d.dirty());
        d.undo();
        assert!(d.edits.annotations.is_empty());
        d.redo();
        assert_eq!(d.edits.annotations.len(), 1);
        let bytes = d.serialize().unwrap();
        let d2 = DocumentModel::open(bytes, None).unwrap();
        assert_eq!(d2.edits.annotations.len(), 1);
        assert_eq!(
            d2.fields.iter().find(|f| f.name == "Name").unwrap().value,
            FieldValue::Text("Alice Smith".into())
        );
        assert_eq!(
            d2.fields.iter().find(|f| f.name == "Radio").unwrap().value,
            FieldValue::Button("Two".into())
        );
        assert_eq!(
            d2.fields
                .iter()
                .find(|f| f.name == "Country")
                .unwrap()
                .value,
            FieldValue::Choice(vec!["Japan".into()])
        );
    }
    #[test]
    fn readonly_and_unicode_refused_without_loss() {
        let mut d = DocumentModel::open(sample(1, true), None).unwrap();
        let id = d.fields.iter().find(|f| f.readonly).unwrap().id;
        d.change(|e| {
            e.fields.insert(id, FieldValue::Text("changed".into()));
        })
        .unwrap();
        assert!(d.serialize().is_err());
        d.undo();
        assert!(!d.dirty());
    }
}
