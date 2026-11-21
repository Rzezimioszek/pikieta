<?xml version="1.0" encoding="UTF-8"?>
<!-- PLIK SYNTETYCZNY do testów Pikiety. Struktura wzorowana na eksporcie EGiB (GML 3.2),
     dane wymyślone: identyfikatory, numery i współrzędne nie odpowiadają żadnemu rzeczywistemu obiektowi. -->
<gml:FeatureCollection xmlns:gml="http://www.opengis.net/gml/3.2"
    xmlns:egb="ewidencjaGruntowIBudynkow:1.0"
    xmlns:xlink="http://www.w3.org/1999/xlink"
    gml:id="TEST.Kolekcja">
  <gml:boundedBy>
    <gml:Envelope srsName="urn:ogc:def:crs:EPSG::2178">
      <gml:lowerCorner>5788300.00 7500700.00</gml:lowerCorner>
      <gml:upperCorner>5788600.00 7501100.00</gml:upperCorner>
    </gml:Envelope>
  </gml:boundedBy>

  <gml:featureMember>
    <egb:EGB_ObrebEwidencyjny gml:id="TEST.Obreb.0101">
      <egb:idIIP>
        <egb:BT_Identyfikator>
          <egb:lokalnyId>obreb-0101</egb:lokalnyId>
          <egb:przestrzenNazw>TEST.EGIB</egb:przestrzenNazw>
          <egb:wersjaId>2024-01-15T10:00:00</egb:wersjaId>
        </egb:BT_Identyfikator>
      </egb:idIIP>
      <egb:idObrebu>999999_9.0101</egb:idObrebu>
      <egb:nazwaWlasna>Obręb Testowy &amp; Przykładowy</egb:nazwaWlasna>
      <egb:geometria>
        <gml:MultiSurface gml:id="TEST.Obreb.0101.g" srsName="urn:ogc:def:crs:EPSG::2178">
          <gml:surfaceMember>
            <gml:Polygon gml:id="TEST.Obreb.0101.g1">
              <gml:exterior>
                <gml:LinearRing>
                  <gml:posList srsDimension="2">5788300.00 7500700.00 5788600.00 7500700.00 5788600.00 7501100.00 5788300.00 7501100.00 5788300.00 7500700.00</gml:posList>
                </gml:LinearRing>
              </gml:exterior>
            </gml:Polygon>
          </gml:surfaceMember>
        </gml:MultiSurface>
      </egb:geometria>
    </egb:EGB_ObrebEwidencyjny>
  </gml:featureMember>

  <gml:featureMember>
    <egb:EGB_DzialkaEwidencyjna gml:id="TEST.Dz.12_1">
      <egb:idIIP>
        <egb:BT_Identyfikator>
          <egb:lokalnyId>dz-12-1</egb:lokalnyId>
          <egb:przestrzenNazw>TEST.EGIB</egb:przestrzenNazw>
        </egb:BT_Identyfikator>
      </egb:idIIP>
      <egb:startObiekt>2019-05-20T08:00:00</egb:startObiekt>
      <egb:idDzialki>999999_9.0101.12/1</egb:idDzialki>
      <egb:geometria>
        <gml:Polygon gml:id="TEST.Dz.12_1.g" srsName="urn:ogc:def:crs:EPSG::2178">
          <gml:exterior>
            <gml:LinearRing>
              <gml:posList srsDimension="2">5788400.00 7500800.00 5788500.00 7500800.00 5788500.00 7500900.00 5788400.00 7500900.00 5788400.00 7500800.00</gml:posList>
            </gml:LinearRing>
          </gml:exterior>
        </gml:Polygon>
      </egb:geometria>
      <egb:numerKW>TE1S/00012345/6</egb:numerKW>
      <egb:poleEwidencyjne>1.0000</egb:poleEwidencyjne>
      <egb:dokladnoscReprezentacjiPola>1</egb:dokladnoscReprezentacjiPola>
      <egb:klasouzytekWGranicachDzialki>
        <egb:EGB_Klasouzytek>
          <egb:OFU>B</egb:OFU>
          <egb:powierzchnia>1.0000</egb:powierzchnia>
        </egb:EGB_Klasouzytek>
      </egb:klasouzytekWGranicachDzialki>
      <egb:lokalizacjaDzialki xlink:href="#TEST.Obreb.0101"/>
      <egb:punktGranicyDzialki xlink:href="#TEST.PG.1"/>
      <egb:punktGranicyDzialki xlink:href="#TEST.PG.2"/>
      <egb:punktGranicyDzialki xlink:href="#TEST.PG.3"/>
      <egb:punktGranicyDzialki xlink:href="#TEST.PG.4"/>
      <egb:JRG2 xlink:href="urn:test:jrg:G.123"/>
      <egb:uwagi><![CDATA[Opis z <nawiasami> & znakami]]></egb:uwagi>
    </egb:EGB_DzialkaEwidencyjna>
  </gml:featureMember>

  <gml:featureMember>
    <egb:EGB_DzialkaEwidencyjna gml:id="TEST.Dz.12_2">
      <egb:idDzialki>999999_9.0101.12/2</egb:idDzialki>
      <egb:geometria>
        <gml:Surface gml:id="TEST.Dz.12_2.g" srsName="urn:ogc:def:crs:EPSG::2178">
          <gml:patches>
            <gml:PolygonPatch>
              <gml:exterior>
                <gml:LinearRing>
                  <gml:posList>5788400.00 7500900.00 5788500.00 7500900.00 5788500.00 7501000.00 5788400.00 7501000.00 5788400.00 7500900.00</gml:posList>
                </gml:LinearRing>
              </gml:exterior>
            </gml:PolygonPatch>
          </gml:patches>
        </gml:Surface>
      </egb:geometria>
      <egb:numerKW/>
      <egb:poleEwidencyjne>1.0000</egb:poleEwidencyjne>
      <egb:lokalizacjaDzialki xlink:href="#TEST.Obreb.0101"/>
      <egb:punktGranicyDzialki xlink:href="#TEST.PG.4"/>
      <egb:punktGranicyDzialki xlink:href="#TEST.PG.3"/>
      <egb:punktGranicyDzialki xlink:href="#TEST.PG.5"/>
      <egb:punktGranicyDzialki xlink:href="#TEST.PG.6"/>
    </egb:EGB_DzialkaEwidencyjna>
  </gml:featureMember>

  <gml:featureMember>
    <egb:EGB_PunktGraniczny gml:id="TEST.PG.1">
      <egb:idPunktu>999999_9.0101.101</egb:idPunktu>
      <egb:geometria>
        <gml:Point gml:id="TEST.PG.1.g" srsName="urn:ogc:def:crs:EPSG::2178">
          <gml:pos>5788400.00 7500800.00</gml:pos>
        </gml:Point>
      </egb:geometria>
      <egb:sposobPozyskania>1</egb:sposobPozyskania>
      <egb:spelnienieWarunkowDokl>1</egb:spelnienieWarunkowDokl>
      <egb:rodzajStabilizacji>1</egb:rodzajStabilizacji>
      <egb:oznWMaterialeZrodlowym>101</egb:oznWMaterialeZrodlowym>
      <egb:numerOperatuTechnicznego>P.9999.2019.123</egb:numerOperatuTechnicznego>
    </egb:EGB_PunktGraniczny>
  </gml:featureMember>
  <gml:featureMember>
    <egb:EGB_PunktGraniczny gml:id="TEST.PG.2">
      <egb:idPunktu>999999_9.0101.102</egb:idPunktu>
      <egb:geometria><gml:Point gml:id="TEST.PG.2.g" srsName="urn:ogc:def:crs:EPSG::2178"><gml:pos>5788500.00 7500800.00</gml:pos></gml:Point></egb:geometria>
      <egb:sposobPozyskania>1</egb:sposobPozyskania>
      <egb:spelnienieWarunkowDokl>1</egb:spelnienieWarunkowDokl>
      <egb:rodzajStabilizacji>2</egb:rodzajStabilizacji>
    </egb:EGB_PunktGraniczny>
  </gml:featureMember>
  <gml:featureMember>
    <egb:EGB_PunktGraniczny gml:id="TEST.PG.3">
      <egb:idPunktu>999999_9.0101.103</egb:idPunktu>
      <egb:geometria><gml:Point gml:id="TEST.PG.3.g" srsName="urn:ogc:def:crs:EPSG::2178"><gml:pos>5788500.00 7500900.00</gml:pos></gml:Point></egb:geometria>
      <egb:sposobPozyskania>2</egb:sposobPozyskania>
      <egb:spelnienieWarunkowDokl>2</egb:spelnienieWarunkowDokl>
    </egb:EGB_PunktGraniczny>
  </gml:featureMember>
  <gml:featureMember>
    <egb:EGB_PunktGraniczny gml:id="TEST.PG.4">
      <egb:idPunktu>999999_9.0101.104</egb:idPunktu>
      <egb:geometria><gml:Point gml:id="TEST.PG.4.g" srsName="urn:ogc:def:crs:EPSG::2178"><gml:pos>5788400.00 7500900.00</gml:pos></gml:Point></egb:geometria>
      <egb:sposobPozyskania>1</egb:sposobPozyskania>
    </egb:EGB_PunktGraniczny>
  </gml:featureMember>
  <gml:featureMember>
    <egb:EGB_PunktGraniczny gml:id="TEST.PG.5">
      <egb:idPunktu>999999_9.0101.105</egb:idPunktu>
      <egb:geometria><gml:Point gml:id="TEST.PG.5.g" srsName="urn:ogc:def:crs:EPSG::2178"><gml:pos>5788500.00 7501000.00</gml:pos></gml:Point></egb:geometria>
    </egb:EGB_PunktGraniczny>
  </gml:featureMember>
  <gml:featureMember>
    <egb:EGB_PunktGraniczny gml:id="TEST.PG.6">
      <egb:idPunktu>999999_9.0101.106</egb:idPunktu>
      <egb:geometria><gml:Point gml:id="TEST.PG.6.g" srsName="urn:ogc:def:crs:EPSG::2178"><gml:pos>5788400.00 7501000.00</gml:pos></gml:Point></egb:geometria>
    </egb:EGB_PunktGraniczny>
  </gml:featureMember>

  <gml:featureMember>
    <egb:EGB_Budynek gml:id="TEST.Bud.1">
      <egb:idBudynku>999999_9.0101.12/1.1_BUD</egb:idBudynku>
      <egb:geometria>
        <gml:Polygon gml:id="TEST.Bud.1.g" srsName="urn:ogc:def:crs:EPSG::2178">
          <gml:exterior>
            <gml:LinearRing>
              <gml:posList>5788420.00 7500820.00 5788460.00 7500820.00 5788460.00 7500860.00 5788420.00 7500860.00 5788420.00 7500820.00</gml:posList>
            </gml:LinearRing>
          </gml:exterior>
          <gml:interior>
            <gml:LinearRing>
              <gml:posList>5788435.00 7500835.00 5788435.00 7500845.00 5788445.00 7500845.00 5788445.00 7500835.00 5788435.00 7500835.00</gml:posList>
            </gml:LinearRing>
          </gml:interior>
        </gml:Polygon>
      </egb:geometria>
      <egb:rodzajWgKST>110</egb:rodzajWgKST>
      <egb:liczbaKondygnacjiNadziemnych>3</egb:liczbaKondygnacjiNadziemnych>
      <egb:dzialkaZabudowana xlink:href="#TEST.Dz.12_1"/>
    </egb:EGB_Budynek>
  </gml:featureMember>

  <gml:featureMember>
    <egb:EGB_KonturUzytkuGruntowego gml:id="TEST.KU.1">
      <egb:idUzytku>999999_9.0101.12/2.1</egb:idUzytku>
      <egb:geometria>
        <gml:Polygon gml:id="TEST.KU.1.g" srsName="urn:ogc:def:crs:EPSG::2178">
          <gml:exterior>
            <gml:Ring>
              <gml:curveMember>
                <gml:LineString gml:id="TEST.KU.1.c1">
                  <gml:posList>5788410.00 7500910.00 5788490.00 7500910.00 5788490.00 7500990.00</gml:posList>
                </gml:LineString>
              </gml:curveMember>
              <gml:curveMember>
                <gml:Curve gml:id="TEST.KU.1.c2">
                  <gml:segments>
                    <gml:Arc>
                      <gml:posList>5788490.00 7500990.00 5788450.00 7500980.00 5788410.00 7500990.00</gml:posList>
                    </gml:Arc>
                    <gml:LineStringSegment>
                      <gml:posList>5788410.00 7500990.00 5788410.00 7500910.00</gml:posList>
                    </gml:LineStringSegment>
                  </gml:segments>
                </gml:Curve>
              </gml:curveMember>
            </gml:Ring>
          </gml:exterior>
        </gml:Polygon>
      </egb:geometria>
      <egb:OFU>R</egb:OFU>
    </egb:EGB_KonturUzytkuGruntowego>
  </gml:featureMember>

  <gml:featureMember>
    <egb:EGB_KonturKlasyfikacyjny gml:id="TEST.KK.1">
      <egb:idKonturu>999999_9.0101.RIIIa</egb:idKonturu>
      <egb:geometria>
        <gml:Polygon gml:id="TEST.KK.1.g" srsName="urn:ogc:def:crs:EPSG::2178" srsDimension="3">
          <gml:exterior>
            <gml:LinearRing>
              <gml:posList>5788400.00 7500900.00 101.5 5788500.00 7500900.00 101.7 5788500.00 7501000.00 102.0 5788400.00 7501000.00 101.9 5788400.00 7500900.00 101.5</gml:posList>
            </gml:LinearRing>
          </gml:exterior>
        </gml:Polygon>
      </egb:geometria>
      <egb:OZU>R</egb:OZU>
      <egb:OZK>IIIa</egb:OZK>
    </egb:EGB_KonturKlasyfikacyjny>
  </gml:featureMember>
</gml:FeatureCollection>
